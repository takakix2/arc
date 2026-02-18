# Roadmap

> arc development follows the **Flux Protocol** — each phase builds on immutable foundations.

---

## Phase 1: Core Engine ✅

**Status**: Complete
**Goal**: Prove that Event Sourcing works for CLI tools.

- [x] `FluxProject` struct with init/open lifecycle
- [x] Signal schema: `id` (UUID v7), `type`, `payload` (JSON Value), `timestamp`
- [x] Append-only NDJSON log (`.flux/signals.jsonl`)
- [x] `arc init` — project initialization with duplicate prevention
- [x] `arc exec` — command execution with start/end correlation (`ref_id`)
- [x] `arc state` — human-readable signal table
- [x] `arc state --json` — machine-readable JSON output
- [x] Performance tracking (`duration_ms` on every execution)
- [x] Unicode-safe display truncation
- [x] stderr/stdout separation (data on stdout, status on stderr)

---

## Phase 2: State Machine 🔜

**Status**: Next
**Goal**: Transform raw signal logs into meaningful environment states.

### 2.1 State Definition

Define what "state" means:

```rust
pub struct FluxState {
    /// Project metadata
    pub project: ProjectInfo,
    /// History of executed commands
    pub executions: Vec<Execution>,
    /// Derived environment snapshot
    pub environment: HashMap<String, serde_json::Value>,
    /// Last known good state
    pub checkpoint: Option<String>,  // Signal ID
}
```

### 2.2 State Reconstruction

Replay signals to build state:

```bash
$ arc state --full
🦄 Flux State (reconstructed from 47 signals)

Project: my_ruby_app (initialized 2026-02-18)
Last operation: bundle install (success, 3.4s ago)

Executions:
  ✅ bundle install    — 3420ms (2 times)
  ✅ rails db:migrate  — 1205ms (1 time)
  ❌ rails test        — 892ms  (1 time, FAILED)

Environment:
  ruby: 3.4.1
  bundler: 2.6.0
  rails: 8.0.1
```

### 2.3 Checkpoints

```bash
# Save current state as a named checkpoint
$ arc checkpoint save "before-upgrade"

# List checkpoints
$ arc checkpoint list

# Diff against checkpoint
$ arc checkpoint diff "before-upgrade"
```

### Deliverables

- [ ] `FluxState` struct and reconstruction logic
- [ ] `arc state --full` (rich state display)
- [ ] Signal filtering (`--type`, `--since`, `--until`)
- [ ] Checkpoint create/list/diff
- [ ] Custom state reducers (user-defined signal → state rules)

---

## Phase 3: Replay Engine 📋

**Status**: Planned
**Goal**: Reproduce any environment from its signal log.

### 3.1 Basic Replay

```bash
# Replay signals on a fresh machine
$ arc replay signals.jsonl

# Dry-run mode: show what would be executed
$ arc replay signals.jsonl --dry-run

# Replay only specific signal types
$ arc replay signals.jsonl --type exec_start
```

### 3.2 Diff & Drift Detection

```bash
# Compare current state against a signal log
$ arc diff production.jsonl

  Missing:
    - bundle install (rails 8.0.1)
    - rails db:migrate (20260218_create_users)
  
  Extra:
    - npm install (not in production log)
```

### 3.3 Export Formats

```bash
# Export as shell script
$ arc export --format shell > setup.sh

# Export as Dockerfile
$ arc export --format dockerfile > Dockerfile

# Export as GitHub Action
$ arc export --format github-action > .github/workflows/setup.yml
```

### Deliverables

- [ ] `arc replay` command with dry-run support
- [ ] `arc diff` for drift detection
- [ ] `arc export` to shell / Dockerfile / CI formats
- [ ] Selective replay (by type, time range, signal ID)
- [ ] Parallel replay (independent operations executed concurrently)

---

## Phase 4: Ecosystem 📋

**Status**: Planned
**Goal**: Make Flux Core available to the wider developer community.

### 4.1 Standalone Crate

```toml
# Anyone can use Flux Core in their Rust project
[dependencies]
flux-core = "0.1"
```

```rust
use flux_core::FluxProject;
use serde_json::json;

let project = FluxProject::init(".")?;
project.record("deploy", json!({"target": "production"}))?;
```

### 4.2 Language Bindings

- **Python**: `pip install flux-core` (PyO3 bindings)
- **Node.js**: `npm install @flux-core/node` (napi-rs bindings)
- **Ruby**: `gem install flux-core` (native extension)

### 4.3 Editor & Agent Integrations

- **VS Code Extension**: Visualize signal timeline, inline state display
- **MCP Server**: AI agents can record/query signals directly
- **GitHub Action**: Automatically record CI/CD operations

### 4.4 Flux Protocol Specification

Formalize the signal schema and state machine rules as an open specification. Enable interoperability between different Flux-compatible tools.

### Deliverables

- [ ] Extract `flux-core` crate and publish to crates.io
- [ ] Python bindings (PyO3)
- [ ] Node.js bindings (napi-rs)
- [ ] VS Code extension (signal timeline visualization)
- [ ] MCP Server for AI agent integration
- [ ] Flux Protocol v1 specification document

---

## Phase 5: Showcase Applications 📋

**Status**: Planned
**Goal**: Demonstrate Flux Core's versatility through real-world tools.

### 5.1 arc (Ruby/General)

The original showcase. Wrap any CLI workflow with structured logging.

### 5.2 flux-ci

CI/CD operation recorder. Capture every step of a build pipeline.

```yaml
# .github/workflows/build.yml
- run: flux-ci exec npm test
- run: flux-ci exec npm run build
- uses: flux-ci/upload-signals@v1
```

### 5.3 flux-dev

Development environment bootstrapper. Share `.flux/signals.jsonl` in your repo, and any contributor can replay to set up their environment.

```bash
# New contributor joins the project
$ git clone repo && cd repo
$ flux-dev replay
# → Automatically installs correct language version, dependencies, etc.
```

---

## Design Principles

Throughout all phases, we adhere to:

1. **Append-Only**: Signals are never modified or deleted
2. **Zero Config**: A single directory (`.flux/`) is all you need
3. **Human + Machine**: Readable by `cat`, parseable by `jq`
4. **Language Agnostic**: Works with any tech stack
5. **Composable**: Small tools that work together (Unix philosophy)

---

## Contributing

arc is in early development. Contributions, ideas, and feedback are welcome.

- **Issues**: Bug reports and feature requests
- **Discussions**: Architecture decisions and design philosophy
- **PRs**: Code contributions (please open an issue first for major changes)

---

*Every operation has meaning. Every state tells a story.*
