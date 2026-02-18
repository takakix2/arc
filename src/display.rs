use anyhow::Result;
use std::path::Path;

use crate::gemfile;
use crate::signals;
use crate::state::FluxState;

// ─────────────────────────────────────────────
// 表示エントリポイント
// ─────────────────────────────────────────────

/// Signal ログを生テーブルで表示する。
pub fn render_raw(signals: &[&signals::Signal], flux_dir: &Path) -> Result<()> {
    eprintln!(
        "🦄 Flux Signals — {} entries from {:?}",
        signals.len(),
        flux_dir
    );

    let sep_top = "┌─────────────┬──────────────────────────────────────┬──────────────────────────────────────────────────┐";
    let sep_mid = "├─────────────┼──────────────────────────────────────┼──────────────────────────────────────────────────┤";
    let sep_bot = "└─────────────┴──────────────────────────────────────┴──────────────────────────────────────────────────┘";

    println!("{sep_top}");
    println!("│ {:<11} │ {:<36} │ {:<48} │", "Type", "ID", "Payload");
    println!("{sep_mid}");

    for s in signals {
        let payload = signals::truncate_display(&s.payload.to_string(), 48);
        println!("│ {:<11} │ {:<36} │ {:<48} │", s.r_type, s.id, payload);
    }

    println!("{sep_bot}");
    Ok(())
}

/// Signal ログから状態を再構築し、サマリーとコマンド統計を表示する。
///
/// `cwd` はプロジェクトルートの絶対パス。Gemfile の読み取りに使用する。
pub fn render_full(signals: &[signals::Signal], cwd: &Path) -> Result<()> {
    let state = FluxState::from_signals(signals);
    let stats = state.command_stats();
    let failed = state.failed_executions();

    // ── ヘッダー ──────────────────────────────
    eprintln!("⚡ Flux State");
    eprintln!();

    if let Some(ref path) = state.project_path {
        eprintln!("  Project:     {}", path);
    }
    if let Some(ref ts) = state.initialized_at {
        eprintln!("  Initialized: {}", fmt_timestamp(ts));
    }
    eprintln!("  Signals:     {}", state.signal_count);
    eprintln!("  Executions:  {}", state.executions.len());

    if let Some(last) = state.last_execution() {
        let icon = if last.success { "✅" } else { "❌" };
        let dur = last.duration_ms.map(fmt_duration).unwrap_or_else(|| "⏳ running".to_string());
        eprintln!("  Last:        {} {} ({})", icon, fmt_cmd(&last.command, &last.args), dur);
    }

    // ── 依存関係 (Gemfile) ──────────────────
    // cwd を基準にした絶対パスで読み取る（相対パス依存を排除）
    let gemfile_path = cwd.join("Gemfile");
    if let Ok(gems) = gemfile::parse(&gemfile_path) {
        if !gems.is_empty() {
            eprintln!();
            eprintln!("  Dependencies ({}):", gems.len());
            for gem in &gems {
                match &gem.version {
                    Some(v) => eprintln!("    📦 {} ({})", gem.name, v),
                    None    => eprintln!("    📦 {}", gem.name),
                }
            }
        }
    }

    // ── コマンド統計テーブル ──────────────────
    if !stats.is_empty() {
        eprintln!();
        let sep_top = "┌──────────────────────────┬───────┬──────────┬──────────┬──────────────┐";
        let sep_mid = "├──────────────────────────┼───────┼──────────┼──────────┼──────────────┤";
        let sep_bot = "└──────────────────────────┴───────┴──────────┴──────────┴──────────────┘";

        println!("{sep_top}");
        println!("│ {:<24} │ {:<5} │ {:<8} │ {:<8} │ {:<12} │", "Command", "Runs", "Success", "Failed", "Avg Time");
        println!("{sep_mid}");

        for stat in &stats {
            let avg = stat.avg_duration_ms.map(fmt_duration).unwrap_or_else(|| "—".to_string());
            let ok  = format!("✅ {}", stat.successes);
            let ng  = if stat.failures > 0 { format!("❌ {}", stat.failures) } else { "—".to_string() };
            println!(
                "│ {:<24} │ {:<5} │ {:<8} │ {:<8} │ {:<12} │",
                signals::truncate_display(&stat.command, 24),
                stat.total_runs, ok, ng, avg
            );
        }

        println!("{sep_bot}");
    }

    // ── 失敗一覧 ─────────────────────────────
    if !failed.is_empty() {
        eprintln!();
        eprintln!("⚠️  Failed Operations ({}):", failed.len());
        for exec in &failed {
            let exit = exec.exit_code.map(|c| c.to_string()).unwrap_or_else(|| "?".to_string());
            let dur  = exec.duration_ms.map(fmt_duration).unwrap_or_else(|| "incomplete".to_string());
            eprintln!("   ❌ {} (exit: {}, {})", fmt_cmd(&exec.command, &exec.args), exit, dur);
        }
    }

    Ok(())
}

/// 直近の操作による差分を表示する。
pub fn render_diff(signals: &[signals::Signal]) -> Result<()> {
    if signals.is_empty() {
        eprintln!("No signals found.");
        return Ok(());
    }

    // 最新の「意味のある」シグナルを探す（exec/install の開始終了ではなくメタデータ系のみ）
    let last = signals.iter()
        .filter(|s| matches!(s.r_type.as_str(), "add" | "remove" | "undo" | "bootstrap" | "init"))
        .last();

    let last = match last {
        Some(s) => s,
        None => {
            eprintln!("No reversible operations found.");
            return Ok(());
        }
    };

    eprintln!("🔍 Last Project Change:");
    eprintln!();

    match last.r_type.as_str() {
        "add" => {
            let gem = last.payload["gem"].as_str().unwrap_or("?");
            eprintln!("  Gemfile:");
            match last.payload["version"].as_str() {
                Some(v) => eprintln!("  \x1b[32m+ gem '{}', '{}'\x1b[0m", gem, v),
                None    => eprintln!("  \x1b[32m+ gem '{}'\x1b[0m", gem),
            }
        }
        "remove" => {
            let gem = last.payload["gem"].as_str().unwrap_or("?");
            eprintln!("  Gemfile:");
            eprintln!("  \x1b[31m- gem '{}'\x1b[0m", gem);
        }
        "undo" => {
            let target = last.payload["target_type"].as_str().unwrap_or("?");
            let gem    = last.payload["gem"].as_str().unwrap_or("?");
            eprintln!("  ⏪ Undo of '{}' ({})", target, gem);
        }
        "bootstrap" => {
            let ruby = last.payload["ruby_version"].as_str().unwrap_or("?");
            eprintln!("  Runtime:");
            eprintln!("  \x1b[32m+ Ruby {}\x1b[0m", ruby);
        }
        _ => {
            eprintln!("  Type: {}", last.r_type);
            eprintln!("  Data: {}", last.payload);
        }
    }

    eprintln!();
    eprintln!("  Timestamp: {}", fmt_timestamp(&last.timestamp));
    eprintln!("  Signal ID: {}", last.id);

    Ok(())
}

// ─────────────────────────────────────────────
// フォーマットヘルパー
// ─────────────────────────────────────────────

pub fn fmt_duration(ms: u64) -> String {
    if ms < 1_000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1_000.0)
    } else {
        format!("{}m{}s", ms / 60_000, (ms % 60_000) / 1_000)
    }
}

fn fmt_timestamp(ts: &str) -> String {
    if ts.len() >= 16 { ts[..16].replace('T', " ") } else { ts.to_string() }
}

/// コマンドと引数を人間が読みやすい文字列に整形する。
pub fn fmt_cmd(cmd: &str, args: &[String]) -> String {
    if args.is_empty() { cmd.to_string() } else { format!("{} {}", cmd, args.join(" ")) }
}
