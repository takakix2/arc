use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug)]
pub struct Signal {
    #[serde(rename = "type")]
    pub r_type: String,
    pub payload: String,
    pub timestamp: String,
}

pub fn init(project_root: &Path) -> Result<PathBuf> {
    let arc_dir = project_root.join(".arc");
    if !arc_dir.exists() {
        fs::create_dir_all(&arc_dir).context("Failed to create .arc directory")?;
    }
    Ok(arc_dir)
}

pub fn read_signals(arc_dir: &Path) -> Result<Vec<Signal>> {
    let signal_file = arc_dir.join("signals.jsonl");
    if !signal_file.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(signal_file)?;
    let signals: Result<Vec<Signal>, _> = content
        .lines()
        .map(|line| serde_json::from_str(line))
        .collect();
        
    Ok(signals?)
}

pub fn record(arc_dir: &Path, type_: &str, payload: &str) -> Result<()> {
    let signal_file = arc_dir.join("signals.jsonl");

    let signal = Signal {
        r_type: type_.to_string(),
        payload: payload.to_string(),
        timestamp: Local::now().to_rfc3339(),
    };

    let json = serde_json::to_string(&signal)?;
    
    // Append to file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&signal_file)
        .context("Failed to open signals.jsonl")?;

    writeln!(file, "{}", json)?;
    
    // Also print to stdout for now
    println!("Signal recorded: {} {}", type_, payload);

    Ok(())
}
