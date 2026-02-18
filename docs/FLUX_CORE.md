# Flux Core

**汎用操作ログ記録・再生エンジン**

> "すべての操作に意味を。すべての状態に物語を。"

---

## 概要

Flux Core は、CLI ツールやプロジェクト管理において **すべての操作を構造化されたイベント（Signal）として記録し、任意の時点の状態（State）を再構築する** ためのエンジンです。

Event Sourcing パターンを開発ツールの世界に持ち込み、「何が起きたか」ではなく「なぜその状態になったか」を追跡可能にします。

### 既存ツールとの違い

| 特性 | シェル履歴 | git log | **Flux Core** |
|---|---|---|---|
| 構造化データ | ❌ | △ (コミットメッセージのみ) | ✅ JSON Value |
| 任意ペイロード | ❌ | ❌ | ✅ |
| 状態の再構築 | ❌ | △ (ファイル差分のみ) | ✅ State Machine |
| 言語非依存 | ✅ | ✅ | ✅ |
| プログラマブル | ❌ | △ | ✅ Rust API / NDJSON |

---

## 哲学

### 1. Signal-First Architecture

操作の「結果」ではなく「意図」を記録します。

```
❌ 従来: コマンドを実行 → 結果を確認 → 失敗したら調査
✅ Flux:  コマンドを記録 → 実行 → 結果を記録 → 状態を更新
```

### 2. Append-Only Log

Signal ログ (`signals.jsonl`) は **追記のみ** です。削除・変更はしません。
これにより、完全な監査証跡と時間旅行（任意時点の状態再現）が可能になります。

### 3. Language Agnostic

Flux Core は特定の言語やエコシステムに依存しません。
Ruby, Python, Node.js, Go, Rust — どのプロジェクトでも `.flux/signals.jsonl` を置くだけで使えます。

---

## Signal 仕様 (v1)

### スキーマ

```json
{
  "id": "string (UUID v7)",
  "type": "string",
  "payload": "any (JSON Value)",
  "timestamp": "string (RFC 3339)"
}
```

### フィールド定義

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `id` | `String` (UUID v7) | ✅ | Signal の一意識別子。時系列ソート可能 |
| `type` | `String` | ✅ | Signal の種別。`init`, `exec_start`, `exec_end`, `snapshot` など |
| `payload` | `serde_json::Value` | ✅ | 任意の構造化データ。型は Signal Type に依存 |
| `timestamp` | `String` (RFC 3339) | ✅ | Signal が記録された時刻。ローカルタイムゾーン付き |

### 組み込み Signal Types

#### `init`
プロジェクトの初期化。

```json
{
  "id": "019c6fb8-3aa1-7570-af92-c5ca5d70d7ba",
  "type": "init",
  "payload": {
    "path": "my_project",
    "version": "0.1.0"
  },
  "timestamp": "2026-02-18T16:21:54+09:00"
}
```

#### `exec_start`
外部コマンドの実行開始。

```json
{
  "id": "019c6fb8-3aa7-7f81-a0ce-b82c8ad46c74",
  "type": "exec_start",
  "payload": {
    "command": "bundle",
    "args": ["install"],
    "cwd": "/home/user/my_project"
  },
  "timestamp": "2026-02-18T16:32:55+09:00"
}
```

#### `exec_end`
外部コマンドの実行完了。

```json
{
  "id": "019c6fb8-3aaa-7181-ba2e-43df7d8bb33d",
  "type": "exec_end",
  "payload": {
    "ref_id": "019c6fb8-3aa7-7f81-a0ce-b82c8ad46c74",
    "exit_code": 0,
    "success": true,
    "duration_ms": 3420
  },
  "timestamp": "2026-02-18T16:32:58+09:00"
}
```

#### `snapshot`
環境の完全なスナップショット。（将来実装）

```json
{
  "type": "snapshot",
  "payload": {
    "ruby_version": "3.4.1",
    "gems": {
      "rails": "8.0.1",
      "puma": "6.5.0"
    },
    "env": {
      "RUBY_HOME": "/usr/local/ruby"
    }
  },
  "timestamp": "2026-02-18T17:00:00+09:00"
}
```

#### カスタム Signal
ユーザーは任意の `type` と `payload` を定義できます。

```json
{
  "type": "deploy",
  "payload": {
    "target": "production",
    "commit": "abc123",
    "deployer": "takaki2"
  },
  "timestamp": "2026-02-18T18:00:00+09:00"
}
```

---

## ストレージ

### ディレクトリ構造

```
my_project/
├── .flux/                    # Flux Core のデータディレクトリ
│   ├── signals.jsonl         # Signal ログ（追記のみ、NDJSON形式）
│   └── state.json            # 最新の State スナップショット（将来実装）
├── src/
└── ...
```

> **注**: 現在の `arc` 実装では `.arc/` を使用していますが、
> 独立クレートとしては `.flux/` を標準ディレクトリとします。

### signals.jsonl

- **形式**: NDJSON (Newline Delimited JSON)
- **エンコーディング**: UTF-8
- **書き込み**: Append-Only (追記のみ)
- **読み込み**: 行単位でパース可能（ストリーミング対応）

1行が1つの Signal に対応します。

```
{"type":"init","payload":{"path":"my_project","version":"0.1.0"},"timestamp":"2026-02-18T16:21:54+09:00"}
{"type":"exec_start","payload":{"command":"echo","args":["hello"],"cwd":"/path"},"timestamp":"2026-02-18T16:32:55+09:00"}
{"type":"exec_end","payload":{"exit_code":0,"success":true},"timestamp":"2026-02-18T16:32:55+09:00"}
```

---

## Rust API

### Signal の記録

```rust
use flux_core::{FluxProject, Signal};
use serde_json::json;

// プロジェクトの初期化
let project = FluxProject::init("./my_project")?;

// Signal の記録（任意の Serialize 可能な型）
project.record("deploy", json!({
    "target": "production",
    "commit": "abc123"
}))?;
```

### Signal の読み込み

```rust
// 全 Signal の読み込み
let signals = project.read_signals()?;

// フィルタリング
let exec_signals: Vec<&Signal> = signals
    .iter()
    .filter(|s| s.r_type.starts_with("exec"))
    .collect();
```

### State の再構築（将来実装）

```rust
// Signal ログから State を再構築
let state = project.rebuild_state()?;

// 特定時点の State を再構築
let state_at = project.rebuild_state_at("2026-02-18T16:30:00+09:00")?;
```

---

## ユースケース

### 1. AI エージェントとの協業

AI エージェントが実行した操作を完全に記録し、人間が後から検証できます。

```bash
# AI エージェントが実行
flux exec npm install express
flux exec npm run build

# 人間が後から確認
flux state
# → 何をインストールし、ビルドが成功したかが構造化データで分かる
```

### 2. 環境の再現

Signal ログを別のマシンに持っていけば、同じ手順で環境を再現できます。

```bash
# マシンA で作業
flux exec ruby -v
flux exec gem install rails
flux exec rails new my_app

# マシンB で再現
flux replay signals.jsonl
# → 同じコマンドを同じ順序で実行
```

### 3. デバッグ・監査

「いつ、何が起きたか」を正確に追跡できます。

```bash
# 失敗したコマンドだけを抽出
flux state --filter "exec_end" --where "success=false"
```

### 4. 教育・メンタリング

環境構築の手順を Signal ログとして共有できます。
README に「この signals.jsonl を replay すれば環境が作れます」と書くだけ。

---

## ロードマップ

### Phase 1: Core Engine ✅ (現在)
- [x] Signal の記録 (`record`)
- [x] Signal の読み込み (`read_signals`)
- [x] 構造化ペイロード (`serde_json::Value`)
- [x] CLI ラッパー (`exec` コマンド)

### Phase 2: State Machine
- [ ] State の定義と再構築 (`rebuild_state`)
- [ ] Signal → State 変換ルールの定義
- [ ] 時間旅行（特定時点の State 再現）
- [ ] `snapshot` Signal の実装

### Phase 3: Replay Engine
- [ ] `flux replay` コマンド（Signal ログの再生）
- [ ] Dry-run モード（実行せずに手順を表示）
- [ ] 差分検出（現在の環境と Signal ログの差分）

### Phase 4: Ecosystem
- [ ] crates.io への公開 (`flux-core`)
- [ ] Python / Node.js バインディング
- [ ] VS Code 拡張（Signal の可視化）
- [ ] MCP Server 連携（AI エージェントからの直接記録）

---

## 設計原則

1. **Zero Configuration**: `.flux/` ディレクトリを作るだけで使える
2. **Human Readable**: NDJSON なので `cat` や `jq` で直接読める
3. **Machine Friendly**: 構造化 JSON なのでプログラムから解析しやすい
4. **Append Only**: ログは追記のみ。破壊的操作は存在しない
5. **Language Agnostic**: Rust クレートだが、NDJSON 形式なのでどの言語からも利用可能

---

## デモンストレーション: arc

`arc` は Flux Core の最初のショーケースです。
Ruby ツールチェーン（rv の補完ツール）として、以下を実証します：

- `arc init`: Flux プロジェクトの初期化
- `arc exec <cmd>`: 任意コマンドの記録実行
- `arc state`: Signal ログからの状態表示

```bash
$ arc init my_ruby_app
✨ arc project initialized successfully.

$ cd my_ruby_app
$ arc exec bundle install
🚀 Executing: bundle install
Signal recorded: exec_start {"command":"bundle","args":["install"],"cwd":"/path/to/my_ruby_app"}
# ... bundle install の出力 ...
Signal recorded: exec_end {"exit_code":0,"success":true}

$ arc state
🦄 Loading Flux State...
---------------------------------------------------
Type        | Timestamp                    | Payload
---------------------------------------------------
init        | 2026-02-18T16:21:54+09:00    | {"path":"my_ruby_app","version":"0.1.0"}
exec_start  | 2026-02-18T16:32:55+09:00    | {"command":"bundle","args":["install"],...}
exec_end    | 2026-02-18T16:32:58+09:00    | {"exit_code":0,"success":true}
---------------------------------------------------
```

---

## ライセンス

MIT

---

*Flux Core — すべての操作に意味を。*
