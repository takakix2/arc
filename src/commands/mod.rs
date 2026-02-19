mod runner;

use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;
use std::{env, fs};

use crate::config::ArcConfig;
use crate::display;
use crate::gemfile;
use crate::signals::{FluxProject, SignalType};
use runner::{ArcEnv, build_ld_library_path, inject_isolated_env, ruby_bin};

// ─────────────────────────────────────────────
// 定数
// ─────────────────────────────────────────────

/// Gem が格納されるサブディレクトリ名。
/// `gems/`: ソース本体, `specifications/`: メタデータ, `extensions/`: C拡張バイナリ
const GEM_SUBDIRS: [&str; 3] = ["gems", "specifications", "extensions"];

// ─────────────────────────────────────────────
// 低レベルヘルパー
// ─────────────────────────────────────────────

/// `Path` を UTF-8 文字列に変換する。非 UTF-8 パスでは `Err` を返す。
fn path_str(p: &Path) -> Result<&str> {
    p.to_str().context("パスが UTF-8 ではありません")
}

/// `src` を `dest` へハードリンク優先でコピーする。
/// `cp -al` が失敗した場合（ファイルシステムが異なる等）は `cp -r` にフォールバックする。
fn cp_link_or_copy(src: &Path, dest: &Path) -> Result<()> {
    let ok = matches!(
        std::process::Command::new("cp")
            .args(["-al", path_str(src)?, path_str(dest)?])
            .status(),
        Ok(s) if s.success()
    );

    if !ok {
        let _ = fs::remove_dir_all(dest); // 失敗した残骸を掃除
        let status = std::process::Command::new("cp")
            .args(["-r", path_str(src)?, path_str(dest)?])
            .status()
            .context("cp -r の起動に失敗しました")?;

        if !status.success() {
            anyhow::bail!("コピーに失敗しました: {:?} → {:?}", src, dest);
        }
    }

    Ok(())
}

/// `src_root` 内の各エントリを `dest_root` へハードリンク優先でコピーする。
/// 既に存在するエントリはスキップする（べき等）。
fn sync_gem_dirs(src_root: &Path, dest_root: &Path) -> Result<()> {
    if !src_root.exists() {
        return Ok(());
    }
    fs::create_dir_all(dest_root)?;

    for entry in fs::read_dir(src_root)? {
        let entry = entry?;
        let dest = dest_root.join(entry.file_name());
        if !dest.exists() {
            // ベストエフォート: 個別エントリの失敗は無視して続行
            let _ = cp_link_or_copy(&entry.path(), &dest);
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────
// arc init
// ─────────────────────────────────────────────

pub fn init(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).context("プロジェクトディレクトリの作成に失敗しました")?;
    }

    let project = FluxProject::init(path)?;

    // デフォルト config.toml を生成
    let config = ArcConfig::default();
    config.save(&project.flux_dir)
        .context("config.toml の初期化に失敗しました")?;

    let signal = project.record(
        SignalType::Init,
        json!({
            "path": path,
            "version": env!("CARGO_PKG_VERSION"),
            "ruby_version": config.ruby.version,
        }),
    )?;

    eprintln!("✨ Flux project initialized at {:?}", path);
    eprintln!("   Signal: {} ({})", signal.id, signal.r_type);
    eprintln!("   Ruby:   {} (change with `arc bootstrap <version>`)", config.ruby.version);

    Ok(())
}

// ─────────────────────────────────────────────
// arc state
// ─────────────────────────────────────────────

pub fn state(json_output: bool, raw: bool, diff: bool, type_filter: Option<String>) -> Result<()> {
    let cwd = env::current_dir()?;
    let project = FluxProject::open(&cwd)?;
    let signals = project.read_signals()?;

    let filtered: Vec<_> = match &type_filter {
        Some(t) => signals.iter().filter(|s| s.r_type == *t).collect(),
        None    => signals.iter().collect(),
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&filtered)?);
        return Ok(());
    }

    if raw {
        return display::render_raw(&filtered, &project.flux_dir);
    }

    if diff {
        return display::render_diff(&signals);
    }

    display::render_full(&signals, &cwd)
}

// ─────────────────────────────────────────────
// arc exec
// ─────────────────────────────────────────────

pub fn exec(args: &[String]) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("コマンドを指定してください。Usage: arc exec <command> [args...]");
    }
    let cwd = env::current_dir()?;
    let project = FluxProject::open(&cwd)?;
    let (cmd, cmd_args) = (&args[0], &args[1..]);

    eprintln!("🚀 arc exec: {}", display::fmt_cmd(cmd, cmd_args));

    runner::run_with_flux(
        &project,
        SignalType::ExecStart,
        SignalType::ExecEnd,
        cmd,
        cmd_args,
        &cwd,
        ArcEnv::System,
    )
}

// ─────────────────────────────────────────────
// arc sync
// ─────────────────────────────────────────────

pub fn sync() -> Result<()> {
    let cwd = env::current_dir()?;
    let project = FluxProject::open(&cwd)
        .context("Flux プロジェクトが見つかりません。`arc init` を実行してください。")?;
    install_with(&project, &cwd)
}

/// `FluxProject` を受け取って bundle install を実行する内部ヘルパー。
/// `add`/`remove`/`undo` から再利用することで `FluxProject::open()` の二重呼び出しを防ぐ。
/// 実行前にキャッシュから Gem を復元し、実行後にキャッシュへ保存する。
fn install_with(project: &FluxProject, cwd: &Path) -> Result<()> {
    if !cwd.join("Gemfile").exists() {
        anyhow::bail!("Gemfile が見つかりません。");
    }

    // config.toml から Ruby API バージョンを取得
    let config = ArcConfig::load(&project.flux_dir)?;
    let ruby_api_ver = crate::config::ruby_api_version(&config.ruby.version);

    // 1. キャッシュから既存の Gem を復元 (Binary Install 相当)
    let _ = restore_gems(cwd, &ruby_api_ver);

    eprintln!("⚡ arc: bundle install → {}", crate::signals::ARC_ENV_DIR);

    let args = vec!["install".to_string()];
    runner::run_with_flux(
        project,
        SignalType::InstallStart,
        SignalType::InstallEnd,
        "bundle",
        &args,
        cwd,
        ArcEnv::Isolated,
    )?;

    // 2. 新しく入った Gem をキャッシュに保存 (将来のプロジェクト用)
    let _ = harvest_gems(cwd, &ruby_api_ver);

    Ok(())
}

// ─────────────────────────────────────────────
// Gem キャッシュ (Harvest & Restore)
// ─────────────────────────────────────────────

/// プロジェクト内の Gem をグローバルキャッシュに保存する（ベストエフォート）。
fn harvest_gems(cwd: &Path, ruby_api_ver: &str) -> Result<()> {
    let gem_cache = crate::signals::get_global_gems_dir();
    let local_base = cwd
        .join(crate::signals::ARC_ENV_DIR)
        .join("ruby")
        .join(ruby_api_ver);

    if !local_base.exists() {
        return Ok(());
    }

    for subdir in GEM_SUBDIRS {
        let _ = sync_gem_dirs(&local_base.join(subdir), &gem_cache.join(subdir));
    }
    Ok(())
}

/// グローバルキャッシュからプロジェクト内へ Gem を復元する（ベストエフォート）。
fn restore_gems(cwd: &Path, ruby_api_ver: &str) -> Result<()> {
    let gem_cache = crate::signals::get_global_gems_dir();
    if !gem_cache.exists() {
        return Ok(());
    }

    let local_base = cwd
        .join(crate::signals::ARC_ENV_DIR)
        .join("ruby")
        .join(ruby_api_ver);

    for subdir in GEM_SUBDIRS {
        let _ = sync_gem_dirs(&gem_cache.join(subdir), &local_base.join(subdir));
    }
    Ok(())
}

// ─────────────────────────────────────────────
// arc run
// ─────────────────────────────────────────────

pub fn run(args: &[String]) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("実行するコマンドを指定してください。");
    }
    let cwd = env::current_dir()?;
    let project = FluxProject::open(&cwd)
        .context("Flux プロジェクトが見つかりません。`arc init` を実行してください。")?;

    let (cmd, cmd_args) = (&args[0], &args[1..]);
    runner::run_with_flux(
        &project,
        SignalType::RunStart,
        SignalType::RunEnd,
        cmd,
        cmd_args,
        &cwd,
        ArcEnv::Isolated,
    )
}

// ─────────────────────────────────────────────
// arc env
// ─────────────────────────────────────────────

pub fn env() -> Result<()> {
    let cwd = env::current_dir()?;
    let env_dir = cwd.join(crate::signals::ARC_ENV_DIR);
    let ruby_bin_path = ruby_bin(&env_dir);

    eprintln!("⚡ arc env");
    eprintln!();
    eprintln!("  Project:   {}", cwd.display());
    eprintln!("  ARC_ENV:   {}", env_dir.display());
    eprintln!("  GEM_HOME:  {}", env_dir.display());
    eprintln!("  Ruby:      {}",
        if ruby_bin_path.exists() { ruby_bin_path.display().to_string() }
        else { "(not bootstrapped — run `arc bootstrap`)".to_string() }
    );

    // Ruby バージョンを実際に走らせて表示（共有ライブラリを解決してから実行）
    if ruby_bin_path.exists() {
        let mut cmd = std::process::Command::new(&ruby_bin_path);
        cmd.arg("--version");

        // LD_LIBRARY_PATH を設定 (runner と同じロジックを共有)
        if let Some(ld_path) = build_ld_library_path(&env_dir) {
            cmd.env("LD_LIBRARY_PATH", ld_path);
        }

        if let Ok(o) = cmd.output() {
            let ver = if !o.stdout.is_empty() {
                String::from_utf8_lossy(&o.stdout).to_string()
            } else {
                String::from_utf8_lossy(&o.stderr).to_string()
            };
            eprintln!("  Version:   {}", ver.trim());
        }
    }

    eprintln!();
    Ok(())
}

// ─────────────────────────────────────────────
// arc shell
// ─────────────────────────────────────────────

pub fn shell() -> Result<()> {
    let cwd = env::current_dir()?;
    let project = FluxProject::open(&cwd)
        .context("Flux プロジェクトが見つかりません。`arc init` を実行してください。")?;

    // 起動するシェルを決定: $SHELL > /bin/bash
    let shell_bin = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

    eprintln!("🐚 arc shell: entering isolated environment");
    eprintln!("   Shell:   {}", shell_bin);
    eprintln!("   GEM_HOME: {}", cwd.join(crate::signals::ARC_ENV_DIR).display());
    eprintln!("   Type 'exit' to leave the arc environment.");
    eprintln!();

    let mut command = std::process::Command::new(&shell_bin);
    inject_isolated_env(&mut command, &cwd)?;

    // ARC_SHELL=1 をセットしておくと、PS1 等でカスタマイズできる
    command.env("ARC_SHELL", "1");

    project.record(
        SignalType::Custom("shell_enter".to_string()),
        json!({ "shell": &shell_bin }),
    )?;

    // インタラクティブシェルを起動。ユーザーが exit するまでブロック。
    let status = command
        .status()
        .map_err(|e| anyhow::anyhow!("シェル '{}' の起動に失敗しました: {}", shell_bin, e))?;

    let exit_code = status.code().unwrap_or(0);
    project.record(
        SignalType::Custom("shell_exit".to_string()),
        json!({ "exit_code": exit_code }),
    )?;

    eprintln!();
    eprintln!("🐚 arc shell: exited (code: {})", exit_code);

    Ok(())
}

// ─────────────────────────────────────────────
// arc add
// ─────────────────────────────────────────────

pub fn add(gem_name: &str, version: Option<&str>) -> Result<()> {
    let cwd = env::current_dir()?;
    let project = FluxProject::open(&cwd)
        .context("Flux プロジェクトが見つかりません。`arc init` を実行してください。")?;

    let gemfile_path = cwd.join("Gemfile");
    let added = gemfile::add_gem(&gemfile_path, gem_name, version)?;

    if added {
        eprintln!("➕ Added '{}' to Gemfile", gem_name);
    } else {
        eprintln!("ℹ️  '{}' は既に Gemfile に存在します。スキップします。", gem_name);
        return Ok(()); // 変更なし → install 不要
    }

    project.record(
        SignalType::Add,
        json!({ "gem": gem_name, "version": version }),
    )?;

    install_with(&project, &cwd)
}

// ─────────────────────────────────────────────
// arc remove
// ─────────────────────────────────────────────

pub fn remove(gem_name: &str) -> Result<()> {
    let cwd = env::current_dir()?;
    let project = FluxProject::open(&cwd)
        .context("Flux プロジェクトが見つかりません。`arc init` を実行してください。")?;

    let gemfile_path = cwd.join("Gemfile");
    if !gemfile_path.exists() {
        anyhow::bail!("Gemfile が見つかりません。");
    }

    let removed = gemfile::remove_gem(&gemfile_path, gem_name)?;

    if removed {
        eprintln!("➖ Removed '{}' from Gemfile", gem_name);
    } else {
        eprintln!("ℹ️  '{}' は Gemfile に見つかりませんでした。スキップします。", gem_name);
        return Ok(()); // 変更なし → install 不要
    }

    project.record(
        SignalType::Remove,
        json!({ "gem": gem_name }),
    )?;

    install_with(&project, &cwd)
}

// ─────────────────────────────────────────────
// arc undo (Time Machine)
// ─────────────────────────────────────────────

pub fn undo() -> Result<()> {
    let cwd = env::current_dir()?;
    let project = FluxProject::open(&cwd)?;
    let signals = project.read_signals()?;

    // 既に取り消し済みのシグナル ID を収集する（所有型 String で保持）
    let already_undone: std::collections::HashSet<String> = signals.iter()
        .filter(|s| s.r_type == "undo")
        .filter_map(|s| s.payload["target_id"].as_str().map(String::from))
        .collect();

    // 最新の「未取り消し」の add/remove を探す
    let target = signals.iter().rev().find(|s| {
        (s.r_type == "add" || s.r_type == "remove")
            && !already_undone.contains(&s.id)
    });

    let target = match target {
        Some(s) => s,
        None    => anyhow::bail!("取り消し可能な操作（add/remove）が見つかりません。"),
    };

    let gem_name = target.payload["gem"].as_str()
        .context("シグナルに gem 名が含まれていません。")?;

    eprintln!("⏪ Undo: {}", target.r_type);

    let gemfile_path = cwd.join("Gemfile");
    match target.r_type.as_str() {
        "add" => {
            eprintln!("   Removing '{}' from Gemfile...", gem_name);
            gemfile::remove_gem(&gemfile_path, gem_name)?;
        }
        "remove" => {
            let version = target.payload["version"].as_str();
            eprintln!("   Restoring '{}' to Gemfile...", gem_name);
            gemfile::add_gem(&gemfile_path, gem_name, version)?;
        }
        _ => unreachable!(),
    }

    project.record(
        SignalType::Undo,
        json!({
            "target_id":   target.id,
            "target_type": target.r_type,
            "gem":         gem_name,
        }),
    )?;

    install_with(&project, &cwd)
}

// ─────────────────────────────────────────────
// arc bootstrap (Global Cache 対応)
// ─────────────────────────────────────────────

fn resolve_ruby_id(version: &str) -> String {
    format!("{}-{}-{}", version, env::consts::OS, env::consts::ARCH)
}

fn resolve_ruby_url(version: &str) -> Result<String> {
    let suffix = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64")  => "ubuntu-24.04",
        ("linux", "aarch64") => "ubuntu-24.04-arm64",
        (os, arch) => anyhow::bail!("未対応のプラットフォームです: {} / {}", os, arch),
    };

    Ok(format!(
        "https://github.com/ruby/ruby-builder/releases/download/toolcache/ruby-{}-{}.tar.gz",
        version, suffix
    ))
}

/// `version`: CLI 引数で指定されたバージョン。None の場合は config.toml を参照する。
pub fn bootstrap(version_arg: Option<&str>) -> Result<()> {
    let cwd = env::current_dir()?;
    let project = FluxProject::open(&cwd)
        .context("Flux プロジェクトが見つかりません。`arc init` を実行してください。")?;

    // バージョン解決: 引数 > config.toml の順で優先
    let mut config = ArcConfig::load(&project.flux_dir)?;
    let ruby_version = if let Some(v) = version_arg {
        // 引数で指定された場合は config.toml を更新して永続化
        config.ruby.version = v.to_string();
        config.save(&project.flux_dir)?;
        eprintln!("📝 Ruby version set to {} in .arc/config.toml", v);
        v.to_string()
    } else {
        config.ruby.version.clone()
    };

    let cache_dir = crate::signals::get_global_cache_dir()
        .join("rubies")
        .join(resolve_ruby_id(&ruby_version));
    let ruby_dest = cwd.join(crate::signals::ARC_ENV_DIR).join("ruby_runtime");

    if ruby_dest.exists() {
        eprintln!("ℹ️  Ruby 実行環境は既にプロジェクト内に存在します: {:?}", ruby_dest);
        eprintln!("   バージョンを変更する場合は ruby_runtime を削除してから再実行してください。");
        return Ok(());
    }

    // 1. グローバルキャッシュにあるか確認
    let cache_hit = cache_dir.exists();
    if cache_hit {
        eprintln!("✨ Cache Hit: Ruby {} found in global cache.", ruby_version);
    } else {
        download_ruby_to_cache(&cache_dir, &ruby_version)?;
    }

    // 2. キャッシュからプロジェクトへリンク/コピー
    eprintln!("⚡ Linking Ruby to project environment...");
    let ruby_env_dir = ruby_dest.parent()
        .context("ruby_dest の親ディレクトリが取得できません")?;
    fs::create_dir_all(ruby_env_dir)?;
    cp_link_or_copy(&cache_dir, &ruby_dest)?;

    project.record(
        SignalType::Bootstrap,
        json!({
            "ruby_version": ruby_version,
            "cache_hit":    cache_hit,
            "dest":         ruby_dest.to_string_lossy(),
        }),
    )?;

    eprintln!("✨ Ruby {} bootstrap complete!", ruby_version);
    Ok(())
}

/// Ruby バイナリをダウンロードしてキャッシュディレクトリに展開する。
/// 失敗した場合はキャッシュディレクトリを削除してエラーを返す。
fn download_ruby_to_cache(cache_dir: &Path, ruby_version: &str) -> Result<()> {
    eprintln!("🚀 Cache Miss: Downloading Ruby {} from ruby-builder...", ruby_version);
    fs::create_dir_all(cache_dir).context("キャッシュディレクトリの作成に失敗しました")?;

    let ruby_url = resolve_ruby_url(ruby_version)?;
    let tmp_archive = cache_dir.join("download.tar.gz");

    let curl_ok = std::process::Command::new("curl")
        .args(["-fL", "--progress-bar", "-o", path_str(&tmp_archive)?, &ruby_url])
        .status()
        .context("curl の起動に失敗しました")?
        .success();

    if !curl_ok {
        let _ = fs::remove_dir_all(cache_dir);
        anyhow::bail!("Ruby バイナリのダウンロードに失敗しました。");
    }

    let tar_ok = std::process::Command::new("tar")
        .args([
            "-xzf", path_str(&tmp_archive)?,
            "-C",   path_str(cache_dir)?,
            "--strip-components=1",
        ])
        .status()
        .context("tar の起動に失敗しました")?
        .success();

    let _ = fs::remove_file(&tmp_archive);

    if !tar_ok {
        let _ = fs::remove_dir_all(cache_dir);
        anyhow::bail!("アーカイブの展開に失敗しました。");
    }

    Ok(())
}
