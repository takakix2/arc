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
    }

    Ok(())
}
