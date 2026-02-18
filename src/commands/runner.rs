use anyhow::Result;
use serde_json::json;
use std::env;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use crate::signals::{ARC_ENV_DIR, FluxProject, SignalType};

/// プロセスの環境モード。
/// `Isolated` は `.arc/env` を GEM_HOME として使用し、
/// `System` はシステムの環境変数をそのまま引き継ぐ。
#[derive(Debug, Clone, PartialEq)]
pub enum ArcEnv {
    /// プロジェクト固有の隔離環境 (.arc/env) を使用する
    Isolated,
    /// システムの環境変数をそのまま使用する
    System,
}

/// コマンドを実行し、開始・終了を Flux シグナルとして記録する。
/// `exec`, `install`, `run` の共通ロジックを一元化する。
pub fn run_with_flux(
    project: &FluxProject,
    start_type: SignalType,
    end_type: SignalType,
    cmd: &str,
    args: &[String],
    cwd: &Path,
    env_mode: ArcEnv,
) -> Result<()> {
    // シグナルに記録する環境コンテキスト
    let env_context = match env_mode {
        ArcEnv::Isolated => json!({ "mode": "isolated", "GEM_HOME": ARC_ENV_DIR }),
        ArcEnv::System   => json!({ "mode": "system" }),
    };

    let start_signal = project.record(
        start_type,
        json!({
            "command": cmd,
            "args": args,
            "cwd": cwd.to_string_lossy(),
            "env_context": env_context,
        }),
    )?;

    let mut command = Command::new(cmd);
    command.args(args);

    // 隔離モードの場合、環境変数を注入する
    if env_mode == ArcEnv::Isolated {
        let env_path = cwd.join(ARC_ENV_DIR);
        let gem_home = env_path.to_string_lossy().to_string();
        let ruby_runtime_bin = env_path.join("ruby_runtime").join("bin");
        let ruby_runtime_lib = env_path.join("ruby_runtime").join("lib");

        command.env("GEM_HOME", &gem_home);
        command.env("BUNDLE_PATH", &gem_home);

        // 共有ライブラリのパス設定 (プリコンパイル Ruby 用)
        if ruby_runtime_lib.exists() {
            let new_ld = match env::var_os("LD_LIBRARY_PATH") {
                Some(current) => {
                    let mut paths = vec![ruby_runtime_lib];
                    paths.extend(env::split_paths(&current));
                    env::join_paths(paths)?
                }
                None => ruby_runtime_lib.into_os_string(),
            };
            command.env("LD_LIBRARY_PATH", new_ld);
        }

        // PATH の先頭に .arc/env/ruby_runtime/bin, .arc/env/bin を追加
        // PATH が未設定の環境でも動作するよう None ケースも処理する
        let new_path = {
            let mut new_paths = vec![
                ruby_runtime_bin, // プリコンパイル Ruby を最優先
                env_path.join("bin"),
            ];
            if let Some(current) = env::var_os("PATH") {
                new_paths.extend(env::split_paths(&current));
            }
            env::join_paths(new_paths)?
        };
        command.env("PATH", new_path);
    }

    let timer = Instant::now();
    let status = command
        .status()
        .map_err(|e| anyhow::anyhow!("コマンド '{}' の起動に失敗しました: {}", cmd, e))?;

    let duration_ms = timer.elapsed().as_millis() as u64;
    let exit_code = status.code().unwrap_or(1);

    project.record(
        end_type,
        json!({
            "ref_id": start_signal.id,
            "exit_code": exit_code,
            "success": status.success(),
            "duration_ms": duration_ms,
        }),
    )?;

    if !status.success() {
        // std::process::exit() は Rust の Drop トレイトを呼び出さずに即座に終了する。
        // 現状すべての Signal 記録は完了しているため問題ないが、
        // 将来バッファリングされた書き込みを導入する場合は要注意。
        std::process::exit(exit_code);
    }

    Ok(())
}
