use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod signals;
mod state;

use signals::FluxProject;
use state::FluxState;

#[derive(Parser)]
#[command(name = "arc")]
#[command(about = "Flux Core Showcase — 操作ログ記録・再生エンジン", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 新しい Flux プロジェクトを初期化する
    Init {
        /// プロジェクトパス（ディレクトリ名）
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// 現在の状態を表示する（Flux State）
    State {
        /// JSON 形式で出力する
        #[arg(long)]
        json: bool,
        /// Signal ログの生データを表示する
        #[arg(long)]
        raw: bool,
        /// Signal type でフィルタリング
        #[arg(long, short = 't')]
        r#type: Option<String>,
    },
    /// 任意のコマンドを実行し、結果を記録する
    Exec {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => cmd_init(&path),
        Commands::State { json, raw, r#type } => cmd_state(json, raw, r#type),
        Commands::Exec { command } => cmd_exec(&command),
    }
}

// ─────────────────────────────────────────────
// サブコマンド実装
// ─────────────────────────────────────────────

fn cmd_init(path: &Path) -> Result<()> {
    // Create directory if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(path).context("Failed to create project directory")?;
    }

    let project = FluxProject::init(path)?;

    let signal = project.record(
        "init",
        json!({
            "path": path,
            "version": env!("CARGO_PKG_VERSION")
        }),
    )?;

    eprintln!("✨ Flux project initialized at {:?}", path);
    eprintln!("   Signal: {} ({})", signal.id, signal.r_type);

    Ok(())
}

fn cmd_state(json_output: bool, raw: bool, type_filter: Option<String>) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let project = FluxProject::open(&current_dir)?;
    let signals = project.read_signals()?;

    // フィルタリング
    let filtered: Vec<_> = if let Some(ref t) = type_filter {
        signals.iter().filter(|s| s.r_type == *t).collect()
    } else {
        signals.iter().collect()
    };

    if json_output {
        let json = serde_json::to_string_pretty(&filtered)?;
        println!("{}", json);
        return Ok(());
    }

    if raw {
        return cmd_state_raw(&filtered, &project);
    }

    // デフォルト: リッチ表示 (Phase 2 State Machine)
    cmd_state_full(&signals, &project)
}

fn cmd_state_raw(signals: &[&signals::Signal], project: &FluxProject) -> Result<()> {
    eprintln!(
        "🦄 Flux Signals — {} entries from {:?}",
        signals.len(),
        project.flux_dir
    );
    println!("┌─────────────┬──────────────────────────────────────┬──────────────────────────────────────────────────┐");
    println!(
        "│ {:<11} │ {:<36} │ {:<48} │",
        "Type", "ID", "Payload"
    );
    println!("├─────────────┼──────────────────────────────────────┼──────────────────────────────────────────────────┤");

    for signal in signals {
        let payload_str = signal.payload.to_string();
        let payload_display = signals::truncate_display(&payload_str, 48);
        println!(
            "│ {:<11} │ {:<36} │ {:<48} │",
            signal.r_type, signal.id, payload_display
        );
    }

    println!("└─────────────┴──────────────────────────────────────┴──────────────────────────────────────────────────┘");
    Ok(())
}

fn cmd_state_full(signals: &[signals::Signal], project: &FluxProject) -> Result<()> {
    let state = FluxState::from_signals(signals);
    let stats = state.command_stats();
    let failed = state.failed_executions();

    // ヘッダー
    eprintln!("⚡ Flux State");
    eprintln!();

    // プロジェクト情報
    if let Some(ref path) = state.project_path {
        eprintln!("  Project:     {}", path);
    }
    if let Some(ref ts) = state.initialized_at {
        eprintln!("  Initialized: {}", format_timestamp(ts));
    }
    eprintln!("  Signals:     {}", state.signal_count);
    eprintln!("  Executions:  {}", state.executions.len());

    // 最後の操作
    if let Some(last) = state.last_execution() {
        let status = if last.success { "✅" } else { "❌" };
        let duration = last.duration_ms
            .map(|d| format_duration(d))
            .unwrap_or_else(|| "⏳ running".to_string());
        let full_cmd = format_command(&last.command, &last.args);
        eprintln!("  Last:        {} {} ({})", status, full_cmd, duration);
    }

    // コマンド統計
    if !stats.is_empty() {
        eprintln!();
        println!("┌──────────────────────────┬───────┬──────────┬──────────┬──────────────┐");
        println!(
            "│ {:<24} │ {:<5} │ {:<8} │ {:<8} │ {:<12} │",
            "Command", "Runs", "Success", "Failed", "Avg Time"
        );
        println!("├──────────────────────────┼───────┼──────────┼──────────┼──────────────┤");

        for stat in &stats {
            let avg = stat.avg_duration_ms
                .map(|d| format_duration(d))
                .unwrap_or_else(|| "—".to_string());
            let success_str = format!("✅ {}", stat.successes);
            let fail_str = if stat.failures > 0 {
                format!("❌ {}", stat.failures)
            } else {
                "—".to_string()
            };
            println!(
                "│ {:<24} │ {:<5} │ {:<8} │ {:<8} │ {:<12} │",
                signals::truncate_display(&stat.command, 24),
                stat.total_runs,
                success_str,
                fail_str,
                avg
            );
        }

        println!("└──────────────────────────┴───────┴──────────┴──────────┴──────────────┘");
    }

    // 失敗コマンドの詳細
    if !failed.is_empty() {
        eprintln!();
        eprintln!("⚠️  Failed Operations ({}):", failed.len());
        for exec in &failed {
            let full_cmd = format_command(&exec.command, &exec.args);
            let exit = exec.exit_code.map(|c| c.to_string()).unwrap_or("?".to_string());
            let duration = exec.duration_ms
                .map(|d| format_duration(d))
                .unwrap_or_else(|| "incomplete".to_string());
            eprintln!("   ❌ {} (exit: {}, {})", full_cmd, exit, duration);
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────
// フォーマットヘルパー
// ─────────────────────────────────────────────

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m{}s", mins, secs)
    }
}

fn format_timestamp(ts: &str) -> String {
    // RFC 3339 → 短縮表示 (「2026-02-18 16:21」)
    if ts.len() >= 16 {
        ts[..16].replace('T', " ")
    } else {
        ts.to_string()
    }
}

fn format_command(cmd: &str, args: &[String]) -> String {
    if args.is_empty() {
        cmd.to_string()
    } else {
        format!("{} {}", cmd, args.join(" "))
    }
}

fn cmd_exec(args: &[String]) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("No command provided. Usage: arc exec <command> [args...]");
    }

    let current_dir = std::env::current_dir()?;
    let project = FluxProject::open(&current_dir)?;

    let cmd = &args[0];
    let cmd_args = &args[1..];

    eprintln!("🚀 Executing: {} {}", cmd, cmd_args.join(" "));

    // Record start
    let start_signal = project.record(
        "exec_start",
        json!({
            "command": cmd,
            "args": cmd_args,
            "cwd": current_dir,
        }),
    )?;

    // Execute
    let timer = Instant::now();
    let status = std::process::Command::new(cmd)
        .args(cmd_args)
        .status()
        .with_context(|| format!("Failed to execute: {}", cmd))?;
    let duration_ms = timer.elapsed().as_millis();

    // Record end (linked to start via ref_id)
    project.record(
        "exec_end",
        json!({
            "ref_id": start_signal.id,
            "exit_code": status.code(),
            "success": status.success(),
            "duration_ms": duration_ms,
        }),
    )?;

    eprintln!(
        "✅ Finished in {}ms (exit: {})",
        duration_ms,
        status.code().unwrap_or(-1)
    );

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
