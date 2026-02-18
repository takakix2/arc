use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod signals;
use signals::FluxProject;

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
        Commands::State { json } => cmd_state(json),
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

fn cmd_state(json_output: bool) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let project = FluxProject::open(&current_dir)?;
    let signals = project.read_signals()?;

    if json_output {
        // JSON 出力 — パイプや jq との連携用
        let json = serde_json::to_string_pretty(&signals)?;
        println!("{}", json);
        return Ok(());
    }

    // 人間向け表示
    eprintln!(
        "🦄 Flux State — {} signals from {:?}",
        signals.len(),
        project.flux_dir
    );
    println!("┌─────────────┬──────────────────────────────────────┬──────────────────────────────────────────────────┐");
    println!(
        "│ {:<11} │ {:<36} │ {:<48} │",
        "Type", "ID", "Payload"
    );
    println!("├─────────────┼──────────────────────────────────────┼──────────────────────────────────────────────────┤");

    for signal in &signals {
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
