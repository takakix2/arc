# 🌈🔥 Ruby 版 uv (arc) 正式版 実装ロードマップ

## 実装の 4 つのフェーズ

### 1. The Flux Core (魂の器)
- Ruby/Gem の機能は**一切実装しない**。
- `arc init`, `arc record` などのコマンドで、「ユーザーの操作を Signal(NDJSON) として記録し、State(JSON) を更新する」部分を作成。
- 「最強のログツール」として動くものを作ることで、Flux アーキテクチャの価値を証明。

### 2. The Shell (外郭) - Hybrid Installer Strategy
- **依存解決の委譲**: 複雑な依存解決（Resolver）は既存の `bundle install`（または `bundle lock`）に任せる。
- **高速インストール**: 生成された `Gemfile.lock` を `arc` が読み取り、**Gem パッケージのダウンロード・展開・配置を Rust で並列実行** する。
- これにより、Resolver を自作せずとも「`bundle install` が遅い」という最大の課題（直列ダウンロード・IOボトルネック）を解決できる。uv/pip と同様の戦略。

### 3. The Engine (心臓)
- 独自の依存解決や Ruby インストール機能を実装。
- 速度ボトルネックになる部分（Gem のダウンロード・展開など）を Rust に置き換えていく。

## 🛠 詳細ステップ

1. **CLI (clap enum)**
2. **signals** (writer/reader/validator/registry)
3. **state** (current.json 他)
4. **state machine** (遷移と emit)
5. **history/snapshots/rollback**
6. **ruby** (ruby-install or 自前管理)
7. **deps solver** (bundler 互換 or 独自)
8. **exec** (Ruby 実行環境)
    - `arc run ruby script.rb`
    - PATH の書き換え
    - GEM_HOME の設定
    - sandbox 的な実行
9. **env** (PATH/GEM_HOME 管理)
    - `.arc/env/` の構築
10. **統合** (init / install / run)

## 📌 Signal & State Machine 統合
- すべての遷移で `emit`
- `validator` を通す
- `writer` に `append`

---
*Created as part of the arc technical specification.*
