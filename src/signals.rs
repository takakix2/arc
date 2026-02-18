use anyhow::{bail, Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Flux Core のデータディレクトリ名
const FLUX_DIR: &str = ".flux";
/// Signal ログファイル名
const SIGNAL_FILE: &str = "signals.jsonl";

// ─────────────────────────────────────────────
// Signal (イベント)
// ─────────────────────────────────────────────

/// 構造化された操作イベント。
/// Flux Core のすべてのデータは Signal の追記ログとして保存される。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signal {
    /// Signal の一意識別子 (UUID v7 — 時系列ソート可能)
    pub id: String,
    /// Signal の種別 (例: "init", "exec_start", "exec_end")
    #[serde(rename = "type")]
    pub r_type: String,
    /// 構造化ペイロード（任意の JSON Value）
    pub payload: serde_json::Value,
    /// Signal が記録された時刻 (RFC 3339)
    pub timestamp: String,
}

// ─────────────────────────────────────────────
// FluxProject (プロジェクト)
// ─────────────────────────────────────────────

/// Flux Core プロジェクト。
/// `.flux/` ディレクトリを管理し、Signal の記録・読み込みを行う。
pub struct FluxProject {
    /// プロジェクトルートディレクトリ
    pub root: PathBuf,
    /// `.flux/` ディレクトリのパス
    pub flux_dir: PathBuf,
    /// `signals.jsonl` のパス
    pub signal_file: PathBuf,
}

impl FluxProject {
    /// 新しい Flux プロジェクトを初期化する。
    /// `.flux/` ディレクトリと `signals.jsonl` を作成する。
    /// 既に初期化済みの場合はエラーを返す。
    pub fn init(project_root: &Path) -> Result<Self> {
        let flux_dir = project_root.join(FLUX_DIR);
        let signal_file = flux_dir.join(SIGNAL_FILE);

        if signal_file.exists() {
            bail!(
                "Already initialized: {:?} exists. Use FluxProject::open() instead.",
                signal_file
            );
        }

        fs::create_dir_all(&flux_dir)
            .with_context(|| format!("Failed to create {:?}", flux_dir))?;

        Ok(Self {
            root: project_root.to_path_buf(),
            flux_dir,
            signal_file,
        })
    }

    /// 既存の Flux プロジェクトを開く。
    /// `.flux/` ディレクトリが存在しない場合はエラーを返す。
    pub fn open(project_root: &Path) -> Result<Self> {
        let flux_dir = project_root.join(FLUX_DIR);
        let signal_file = flux_dir.join(SIGNAL_FILE);

        if !flux_dir.exists() {
            bail!(
                "Not a Flux project: {:?} not found. Run `init` first.",
                flux_dir
            );
        }

        Ok(Self {
            root: project_root.to_path_buf(),
            flux_dir,
            signal_file,
        })
    }

    /// Signal を記録し、記録された Signal を返す。
    /// stdout への出力は行わない（呼び出し側の責務）。
    pub fn record<T: Serialize>(&self, type_: &str, payload: T) -> Result<Signal> {
        let signal = Signal {
            id: Uuid::now_v7().to_string(),
            r_type: type_.to_string(),
            payload: serde_json::to_value(payload)?,
            timestamp: Local::now().to_rfc3339(),
        };

        let json = serde_json::to_string(&signal)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.signal_file)
            .with_context(|| format!("Failed to open {:?}", self.signal_file))?;

        writeln!(file, "{}", json)?;

        Ok(signal)
    }

    /// すべての Signal を時系列順に読み込む。
    pub fn read_signals(&self) -> Result<Vec<Signal>> {
        if !self.signal_file.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&self.signal_file)?;
        let mut signals = Vec::new();

        for (i, line) in content.lines().enumerate() {
            let signal: Signal = serde_json::from_str(line)
                .with_context(|| format!("Failed to parse signal at line {}", i + 1))?;
            signals.push(signal);
        }

        Ok(signals)
    }

    /// Signal の総数を返す。
    pub fn signal_count(&self) -> Result<usize> {
        if !self.signal_file.exists() {
            return Ok(0);
        }
        let content = fs::read_to_string(&self.signal_file)?;
        Ok(content.lines().count())
    }
}

// ─────────────────────────────────────────────
// ヘルパー関数
// ─────────────────────────────────────────────

/// 文字列を指定文字数で安全に切り詰める（Unicode 安全）。
pub fn truncate_display(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
