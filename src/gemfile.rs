/// Gemfile の行単位パース・操作ユーティリティ。
///
/// Bundler の DSL は Ruby なので完全なパースは行わない。
/// 実用上の範囲（`gem 'name'` / `gem "name"` / バージョン指定付き）を対象とする。
use std::path::Path;
use anyhow::{Context, Result};

// ─────────────────────────────────────────────
// 型定義
// ─────────────────────────────────────────────

/// Gemfile から解析した Gem エントリ。
#[derive(Debug, Clone)]
pub struct GemEntry {
    pub name: String,
    pub version: Option<String>,
}

// ─────────────────────────────────────────────
// パース
// ─────────────────────────────────────────────

/// Gemfile を読み込み、`gem` 宣言の一覧を返す。
pub fn parse(gemfile: &Path) -> Result<Vec<GemEntry>> {
    let content = std::fs::read_to_string(gemfile)
        .with_context(|| format!("Gemfile の読み込みに失敗しました: {:?}", gemfile))?;
    Ok(parse_content(&content))
}

/// 文字列から `gem` 宣言を解析する（テスト可能な純粋関数）。
pub fn parse_content(content: &str) -> Vec<GemEntry> {
    content
        .lines()
        .filter_map(parse_gem_line)
        .collect()
}

/// 1行を解析して `GemEntry` を返す。
/// 対応フォーマット:
///   gem 'name'
///   gem "name"
///   gem 'name', '~> 1.0'
///   gem 'name', '>= 1.0', '< 2.0'
fn parse_gem_line(line: &str) -> Option<GemEntry> {
    let trimmed = line.trim();

    // コメント行・空行をスキップ
    if trimmed.starts_with('#') || trimmed.is_empty() {
        return None;
    }

    // `gem ` または `gem(` で始まる行のみ対象
    let rest = trimmed
        .strip_prefix("gem ")
        .or_else(|| trimmed.strip_prefix("gem("))?;

    // 最初のクォート内の文字列を Gem 名として取得
    let name = extract_first_quoted(rest)?;

    // バージョン指定: 2番目以降のクォート内文字列（あれば）
    let version = extract_version_specs(rest, &name);

    Some(GemEntry { name, version })
}

/// 文字列から最初のシングル/ダブルクォートで囲まれた部分を抽出する。
fn extract_first_quoted(s: &str) -> Option<String> {
    for quote in ['"', '\''] {
        if let Some(start) = s.find(quote) {
            let inner = &s[start + 1..];
            if let Some(end) = inner.find(quote) {
                return Some(inner[..end].to_string());
            }
        }
    }
    None
}

/// Gem 名の後に続くバージョン指定文字列を抽出する。
/// 例: `gem 'json', '~> 2.0'` → `Some("~> 2.0")`
/// 例: `gem 'rails', '>= 7.0', '< 8.0'` → `Some(">= 7.0, < 8.0")`
fn extract_version_specs(line: &str, gem_name: &str) -> Option<String> {
    // 行中のすべてのクォート文字列を順番に収集する
    let mut quoted_strings: Vec<String> = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch == '\'' || ch == '"' {
            let quote = ch;
            let mut s = String::new();
            for (_, c) in chars.by_ref() {
                if c == quote {
                    break;
                }
                s.push(c);
            }
            quoted_strings.push(s);
        }
    }

    // 最初の要素が Gem 名、2番目以降がバージョン指定
    if quoted_strings.len() <= 1 {
        return None;
    }

    // Gem 名が一致することを確認してから残りを返す
    if quoted_strings[0] != gem_name {
        return None;
    }

    let specs: Vec<&str> = quoted_strings[1..].iter().map(|s| s.as_str()).collect();
    Some(specs.join(", "))
}

// ─────────────────────────────────────────────
// 操作
// ─────────────────────────────────────────────

/// Gemfile に Gem を追加する。既に存在する場合は `false` を返す。
/// 存在チェックは行単位の完全一致（Gem 名が一致する行があるか）で行う。
pub fn add_gem(gemfile: &Path, gem_name: &str, version: Option<&str>) -> Result<bool> {
    let content = if gemfile.exists() {
        std::fs::read_to_string(gemfile)?
    } else {
        "source 'https://rubygems.org'\n".to_string()
    };

    // 行単位の重複チェック（部分一致を防ぐ）
    if parse_content(&content).iter().any(|e| e.name == gem_name) {
        return Ok(false); // 既存
    }

    let new_line = match version {
        Some(v) => format!("gem '{}', '{}'\n", gem_name, v),
        None    => format!("gem '{}'\n", gem_name),
    };

    let new_content = format!("{}\n{}", content.trim_end_matches('\n'), new_line);
    std::fs::write(gemfile, new_content)
        .with_context(|| format!("Gemfile の書き込みに失敗しました: {:?}", gemfile))?;

    Ok(true) // 追加した
}

/// Gemfile から Gem を削除する。削除できた場合は `true` を返す。
pub fn remove_gem(gemfile: &Path, gem_name: &str) -> Result<bool> {
    let content = std::fs::read_to_string(gemfile)
        .with_context(|| format!("Gemfile の読み込みに失敗しました: {:?}", gemfile))?;

    let mut removed = false;
    let new_lines: Vec<&str> = content
        .lines()
        .filter(|line| {
            if let Some(entry) = parse_gem_line(line)
                && entry.name == gem_name {
                    removed = true;
                    return false; // この行を除外
                }
            true
        })
        .collect();

    if removed {
        // 末尾改行を保持
        let mut new_content = new_lines.join("\n");
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        std::fs::write(gemfile, new_content)
            .with_context(|| format!("Gemfile の書き込みに失敗しました: {:?}", gemfile))?;
    }

    Ok(removed)
}

// ─────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let content = "source 'https://rubygems.org'\ngem 'json'\n";
        let gems = parse_content(content);
        assert_eq!(gems.len(), 1);
        assert_eq!(gems[0].name, "json");
        assert!(gems[0].version.is_none());
    }

    #[test]
    fn test_parse_with_version() {
        let content = "gem 'rails', '~> 7.0'\n";
        let gems = parse_content(content);
        assert_eq!(gems[0].name, "rails");
        assert_eq!(gems[0].version.as_deref(), Some("~> 7.0"));
    }

    #[test]
    fn test_parse_double_quote() {
        let content = "gem \"json\"\n";
        let gems = parse_content(content);
        assert_eq!(gems[0].name, "json");
    }

    #[test]
    fn test_no_partial_match() {
        // json-rails は json とは別の Gem
        let content = "gem 'json-rails'\n";
        let gems = parse_content(content);
        assert_eq!(gems.len(), 1);
        assert_eq!(gems[0].name, "json-rails");
        // json という名前の Gem は存在しない
        assert!(!gems.iter().any(|e| e.name == "json"));
    }

    #[test]
    fn test_skip_comments() {
        let content = "# gem 'commented_out'\ngem 'active'\n";
        let gems = parse_content(content);
        assert_eq!(gems.len(), 1);
        assert_eq!(gems[0].name, "active");
    }
}
