use crate::signals::Signal;
use std::collections::HashMap;

// ─────────────────────────────────────────────
// State (Signal ログから再構築される環境状態)
// ─────────────────────────────────────────────

/// 個々のコマンド実行記録（exec_start + exec_end のペア）
#[derive(Debug)]
pub struct Execution {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub exit_code: Option<i64>,
    pub success: bool,
    pub duration_ms: Option<u64>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub start_id: String,
}

/// コマンドごとの集計統計
#[derive(Debug)]
pub struct CommandStats {
    pub command: String,
    pub total_runs: usize,
    pub successes: usize,
    pub failures: usize,
    pub avg_duration_ms: Option<u64>,
    pub last_run: String,
}

/// Signal ログから再構築されたプロジェクト状態
#[derive(Debug)]
pub struct FluxState {
    /// プロジェクトパス
    pub project_path: Option<String>,
    /// arc バージョン
    pub version: Option<String>,
    /// 初期化日時
    pub initialized_at: Option<String>,
    /// 全実行記録
    pub executions: Vec<Execution>,
    /// Signal 総数
    pub signal_count: usize,
}

impl FluxState {
    /// Signal のベクターから State を再構築する
    pub fn from_signals(signals: &[Signal]) -> Self {
        let mut state = FluxState {
            project_path: None,
            version: None,
            initialized_at: None,
            executions: Vec::new(),
            signal_count: signals.len(),
        };

        // exec_start を一時的に保持する HashMap
        let mut pending_starts: HashMap<String, &Signal> = HashMap::new();

        for signal in signals {
            match signal.r_type.as_str() {
                "init" => {
                    state.initialized_at = Some(signal.timestamp.clone());
                    state.project_path = signal.payload.get("path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    state.version = signal.payload.get("version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
                "exec_start" | "install_start" | "run_start" => {
                    // For these start signals, we just store them to match with their corresponding end signals.
                    // The actual logic for active_operation, history_count, etc., is not part of FluxState.
                    pending_starts.insert(signal.id.clone(), signal);
                }
                "exec_end" | "install_end" | "run_end" => {
                    // For these end signals, we process them similarly to exec_end.
                    // The logic for active_operation, last_exit_code, etc., is not part of FluxState.
                    let ref_id = signal.payload.get("ref_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let start_signal = pending_starts.remove(ref_id);

                    let (command, args, cwd, started_at, start_id) = if let Some(start) = start_signal {
                        let cmd = start.payload.get("command")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let args = start.payload.get("args")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        let cwd = start.payload.get("cwd")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        (cmd, args, cwd, start.timestamp.clone(), start.id.clone())
                    } else {
                        ("unknown".to_string(), vec![], String::new(), String::new(), String::new())
                    };

                    let exit_code = signal.payload.get("exit_code")
                        .and_then(|v| v.as_i64());
                    let success = signal.payload.get("success")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let duration_ms = signal.payload.get("duration_ms")
                        .and_then(|v| v.as_u64());

                    state.executions.push(Execution {
                        command,
                        args,
                        cwd,
                        exit_code,
                        success,
                        duration_ms,
                        started_at,
                        ended_at: Some(signal.timestamp.clone()),
                        start_id,
                    });
                }
                _ => {
                    // Unknown signal types are ignored (forward compatibility)
                }
            }
        }

        // 未完了の exec_start (SIGKILL 等で exec_end がない) を orphan として記録
        for (_id, start) in pending_starts {
            let cmd = start.payload.get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let args = start.payload.get("args")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let cwd = start.payload.get("cwd")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            state.executions.push(Execution {
                command: cmd,
                args,
                cwd,
                exit_code: None,
                success: false,
                duration_ms: None,
                started_at: start.timestamp.clone(),
                ended_at: None,
                start_id: start.id.clone(),
            });
        }

        state
    }

    /// コマンドごとの統計を計算する
    pub fn command_stats(&self) -> Vec<CommandStats> {
        let mut stats_map: HashMap<String, Vec<&Execution>> = HashMap::new();

        for exec in &self.executions {
            stats_map.entry(exec.command.clone()).or_default().push(exec);
        }

        let mut stats: Vec<CommandStats> = stats_map
            .into_iter()
            .map(|(command, execs)| {
                let total_runs = execs.len();
                let successes = execs.iter().filter(|e| e.success).count();
                let failures = total_runs - successes;

                let durations: Vec<u64> = execs.iter()
                    .filter_map(|e| e.duration_ms)
                    .collect();
                let avg_duration_ms = if durations.is_empty() {
                    None
                } else {
                    Some(durations.iter().sum::<u64>() / durations.len() as u64)
                };

                let last_run = execs.iter()
                    .max_by_key(|e| &e.started_at)
                    .map(|e| e.started_at.clone())
                    .unwrap_or_default();

                CommandStats {
                    command,
                    total_runs,
                    successes,
                    failures,
                    avg_duration_ms,
                    last_run,
                }
            })
            .collect();

        // 最新のコマンドが上に来るようにソート
        stats.sort_by(|a, b| b.last_run.cmp(&a.last_run));
        stats
    }

    /// 最後に実行されたコマンド
    pub fn last_execution(&self) -> Option<&Execution> {
        self.executions.last()
    }

    /// 失敗したコマンドの一覧
    pub fn failed_executions(&self) -> Vec<&Execution> {
        self.executions.iter().filter(|e| !e.success).collect()
    }
}
