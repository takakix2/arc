🌈🔥 arc は「Ruby 版 uv」である — 正式な言語化
---

arc は Ruby のための “uv 的アプローチ” を実現する新しいツールチェーンである。

uv が Python に対して行ったこと——

- 仮想環境の自動管理
- 依存解決とインストールの高速化
- 実行環境の完全な再現性
- すべてを 1 つのツールに統合
- Rust による圧倒的な速度
- ローカルに閉じた安全な実行モデル

すべてを Ruby の世界に持ち込むのが **arc**。

つまり arc は：

- Ruby のための高速・一体型・再現性重視のツールチェーン。
- Ruby の “uv” として設計されている。

## 🔥 Ruby 版 uv としての arc の特徴（対応表）

| uv（Python） | arc（Ruby） | 説明 |
| :--- | :--- | :--- |
| 仮想環境を自動生成 | .arc/env を自動生成 | Ruby の PATH / GEM_HOME を完全管理 |
| 依存解決を高速化 | deps solver（bundler 互換 or 独自） | lockfile を高速に扱う |
| パッケージを高速インストール | gems install | RubyGems を Rust で高速化 |
| Python 実行を隔離 | arc exec / arc run | Ruby 実行環境を sandbox 化 |
| Rust 製で高速 | Rust 製で高速 | Ruby の重い部分を Rust が肩代わり |
| 1 ツールで完結 | arc 1 つで完結 | ruby-install / bundler / rbenv を統合 |

## 🌟 uv にはない arc の独自性

arc は uv の単なる模倣ではなく、**flux プロトコル × state machine × snapshots** という “意味のあるシステム” として進化している。

1. **すべての動作が NDJSON（火花）として記録される**
   - uv にはない「完全なイベントログ」
2. **state machine による儀式的な動作**
   - init → solve → install → run の流れが明確
3. **snapshots / rollback による物語性**
   - Ruby の環境を「巻き戻せる」
4. **registry / validator による意味の保証**
   - イベントの意味を schema で定義
5. **arc の世界観（火花・光・物語）がある**
   - uv よりも “意味の層” が厚い

## 🔥 キャッチコピー

**arc — Ruby のための高速・一体型ツールチェーン。**
**Rust 製の “Ruby 版 uv”。**
すべての動作が火花（signals）として記録され、環境は光（state）として可視化され、物語（history）として残る。

---
*arc is a next-generation Ruby toolchain powered by Flux Protocol.*
