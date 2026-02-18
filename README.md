# arc

**Flux Core のショーケース — Ruby ツールチェーンとしての実証実装**

> "すべての操作に意味を。すべての状態に物語を。"

---

## arc とは

arc は **[Flux Core](docs/FLUX_CORE.md)** の設計実証を兼ねた Ruby ツールチェーンです。

### Flux Core

Flux Core は **汎用操作ログ記録・再生エンジン** です。
Event Sourcing パターンを開発ツールの世界に持ち込み、CLI で実行されるあらゆる操作を構造化イベント（Signal）として記録します。

- 📝 **Signal**: すべての操作を構造化 JSON で記録
- 🦄 **State**: Signal ログから任意時点の環境状態を再構築
- 🔄 **Replay**: Signal ログを別環境で再生し、環境を再現

詳細は **[Flux Core ドキュメント](docs/FLUX_CORE.md)** を参照。

### なぜ Ruby？

Ruby エコシステムには [`rv`](https://github.com/nicholaides/rv)（Bundler/rbenv のコアメンテナーによる Rust 製 Ruby マネージャー）が登場しています。
arc は rv と**競合**するのではなく、**補完**する存在です。

| ツール | 役割 |
|---|---|
| **rv** | Ruby バージョン管理 + パッケージ管理（uv 相当） |
| **arc** | 操作の記録・再現・監査（Flux Core のショーケース） |

arc は `rv` や `bundler` の操作を `arc exec` で**ラップして記録**することで、
「何をインストールしたか」「いつビルドが壊れたか」を追跡可能にします。

---

## クイックスタート

```bash
# ビルド
cargo build --release

# プロジェクト初期化
arc init my_ruby_app

# 任意のコマンドを記録実行
cd my_ruby_app
arc exec bundle install
arc exec rails new .

# 状態を確認
arc state
```

---

## コマンド

### `arc init <path>`
Flux プロジェクトを初期化し、`.arc/` ディレクトリを作成します。

### `arc exec <command> [args...]`
任意のコマンドを実行し、開始・終了を Signal として記録します。

### `arc state`
`.arc/signals.jsonl` から Signal ログを読み込み、現在の状態を表示します。

---

## 技術スタック

- **言語**: Rust
- **CLI**: clap v4
- **シリアライゼーション**: serde + serde_json
- **ログ形式**: NDJSON (Newline Delimited JSON)

---

## ロードマップ

- [x] Phase 1: Flux Core (Signal 記録・読み込み・構造化ペイロード)
- [ ] Phase 2: State Machine (Signal → State 変換)
- [ ] Phase 3: Replay Engine (Signal ログの再生)
- [ ] Phase 4: Flux Core の独立クレート化 (`flux-core` on crates.io)

---

## ライセンス

MIT
