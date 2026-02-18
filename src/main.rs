use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

mod signals;

#[derive(Parser)]
#[command(name = "arc")]
#[command(about = "Ruby 版 uv - 次世代 Ruby ツールチェーン", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 新しい arc プロジェクトを初期化する
    Init {
        /// プロジェクト名（ディレクトリ名）
        path: PathBuf,
    },
    /// 現在の環境の状態を表示する（Flux State）
    State,
    /// 任意のコマンドを実行し、結果を記録する（Flux Core 汎用機能）
    Exec {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { path } => {
            println!("Initializing arc project at {:?}", path);
            
            // Create directory if it doesn't exist
            if !path.exists() {
                fs::create_dir_all(path).context("Failed to create project directory")?;
            }

            // Initialize .arc structure
            let arc_dir = signals::init(path)?;
            
            // Record the initialization signal
            signals::record(&arc_dir, "init", &format!("Created project at {:?}", path))?;

            println!("✨ arc project initialized successfully.");
        }
        Commands::State => {
            // Check if current directory is an arc project
            let current_dir = std::env::current_dir()?;
            let arc_dir = current_dir.join(".arc");

            if arc_dir.exists() {
                 println!("🦄 Loading Flux State from {:?}...", arc_dir);
                 let signals = signals::read_signals(&arc_dir)?;
                 
                 println!("---------------------------------------------------");
                 println!("Type        | Timestamp                    | Payload");
                 println!("---------------------------------------------------");
                 for signal in signals {
                     println!("{:<11} | {} | {}", signal.r_type, signal.timestamp, signal.payload);
                 }
                 println!("---------------------------------------------------");
            } else {
                println!("No .arc directory found. Run `arc init <path>` to start.");
            }
        }
        Commands::Exec { command: args } => {
             let current_dir = std::env::current_dir()?;
             let arc_dir = current_dir.join(".arc");
             
             if !arc_dir.exists() {
                 eprintln!("Error: Not an arc project. Run `arc init` first.");
                 std::process::exit(1);
             }

             if args.is_empty() {
                 eprintln!("Error: No command provided.");
                 std::process::exit(1);
             }

             let cmd = &args[0];
             let cmd_args = &args[1..];

             println!("🚀 Executing: {} {}", cmd, cmd_args.join(" "));
             
             // Record start
             signals::record(&arc_dir, "exec_start", &format!("Command: {} {}", cmd, cmd_args.join(" ")))?;

             let status = std::process::Command::new(cmd)
                 .args(cmd_args)
                 .status()
                 .context("Failed to execute command")?;

             // Record end
             signals::record(&arc_dir, "exec_end", &format!("ExitCode: {}", status))?;
             
             if !status.success() {
                 std::process::exit(status.code().unwrap_or(1));
             }
        }
    }

    Ok(())
}
