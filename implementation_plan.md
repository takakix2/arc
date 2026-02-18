# Implementation Plan - arc Phase 2: Ruby Shell Integration

`arc` プロジェクトを「Ruby版 uv」として進化させるための第一歩として、`install` および `run` コマンドを追加し、Ruby エコシステムとの統合を開始します。

## 概要
Flux Core の基本機能を活かしつつ、Ruby のパッケージ管理と実行環境を制御する「外郭（Shell）」を構築します。最初は既存の `bundle` をラップするハイブリッド戦略をとり、徐々に Rust による高速化領域を拡大します。

## 変更内容

### 1. `Cargo.toml` の更新
- 並列処理や HTTP 通信、ファイルパースに必要な依存関係を検討（今回はまず基本統合のため、既存のままか最小限の追加に留める）。
- 追加候補: `tokio`, `reqwest` (将来的な並列ダウンロード用)。

### 2. `src/main.rs`: サブコマンドの追加
- `Install`: `bundle install` を実行し、その成否をシグナルとして記録。
- `Run`: 環境変数（`GEM_HOME`, `PATH` 等）をセットした状態でコマンドを実行。

### 3. `src/signals.rs`: Signal 型の拡張
- `install_start`, `install_end`
- `run_start`, `run_end`
などの Ruby 特有のコンテキストを考慮したシグナルタイプの追加を検討。

### 4. 実装の詳細
- `cmd_install`: `Gemfile` の存在を確認し、`bundle install` を子プロセスとして起動。成否を `record` する。
- `cmd_run`: 実行前に構成済みの `.arc/env` (将来作成予定) 等を環境変数に反映させ、ターゲットコマンドを実行。

## 影響範囲
- 既存の `exec` コマンドには影響を与えません。
- 新規のシグナルが生成されるため、`state.rs` のリデューサー（`apply` 関数）にこれらの新しいシグナルへの対応を追加する必要があります。

## テスト計画
- [ ] `arc install` が正常に `bundle install` を呼び出し、ログに残るか。
- [ ] `arc run ruby -v` 等が正しく動作し、記録されるか。
- [ ] `arc state` でインストールの履歴が表示されるか。
