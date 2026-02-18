<div align="center">

# ⚡ arc

**Every operation has meaning. Every state tells a story.**

[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Flux Core](https://img.shields.io/badge/Powered_by-Flux_Core-blueviolet)](docs/FLUX_CORE.md)

*The first CLI tool built on Event Sourcing.*
*Not just what happened — **why** it happened.*

</div>

---

## The Problem

Every developer has been here:

```
"It was working yesterday."
"What changed?"
"I don't know. I ran some commands."
```

Shell history gives you *commands*. Git gives you *file diffs*. But neither gives you the **full story** — the sequence of environment mutations, their outcomes, their timing, and their relationships.

**arc** records that story.

---

## What is arc?

arc is a CLI wrapper powered by **[Flux Core](docs/FLUX_CORE.md)** — an event sourcing engine for your terminal. Every command you run through arc is captured as a structured, immutable **Signal** in an append-only log.

```bash
# Instead of this:
$ bundle install
$ rails db:migrate
$ rails server
# ↑ Lost to history. No context. No structure.

# Do this:
$ arc exec bundle install
$ arc exec rails db:migrate
$ arc exec rails server
# ↑ Every operation recorded with full context.
```

Then, at any point:

```bash
$ arc state --json | jq '.[] | select(.type == "exec_end" and .payload.success == false)'
# → Instantly find every failed operation, when it happened, how long it took.
```

---

## ✨ Features

### 📝 Structured Signal Logging

Every operation is a first-class event with a unique ID, structured payload, and timestamp.

```json
{
  "id": "019c6fb8-3aa7-7f81-a0ce-b82c8ad46c74",
  "type": "exec_start",
  "payload": {
    "command": "bundle",
    "args": ["install"],
    "cwd": "/home/user/my_app"
  },
  "timestamp": "2026-02-18T16:32:55+09:00"
}
```

### 🔗 Signal Correlation

`exec_end` signals link back to their `exec_start` via `ref_id`, enabling you to reconstruct the full lifecycle of any operation.

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
  "timestamp": "2026-02-18T16:33:39+09:00"
}
```

### ⏱️ Performance Tracking

Every execution is timed. Know exactly how long `bundle install` took, every single time.

### 🤖 AI-Agent Ready

Designed for the age of AI pair programming. When an AI agent runs commands on your behalf, every action is logged with full context — auditable, reviewable, reproducible.

### 🗄️ Machine-Readable Output

```bash
# Human-friendly table
$ arc state

# Machine-friendly JSON (pipe to jq, feed to scripts)
$ arc state --json
```

### 🔒 Append-Only Integrity

Signal logs are **never modified or deleted**. Like a blockchain for your terminal operations.

---

## Quick Start

```bash
# Build from source
git clone https://github.com/yourname/arc.git
cd arc && cargo build --release

# Initialize a project
arc init my_project
cd my_project

# Run commands through arc
arc exec npm install
arc exec npm run build
arc exec npm test

# View the full operation history
arc state

# Get machine-readable output
arc state --json
```

---

## How It Compares

arc doesn't compete with package managers. It **wraps** them.

| | Shell History | git log | rv / uv | **arc** |
|---|:---:|:---:|:---:|:---:|
| Structured data | ❌ | △ | ❌ | ✅ |
| Operation timing | ❌ | ❌ | ❌ | ✅ |
| Signal correlation | ❌ | ❌ | ❌ | ✅ |
| Machine-readable | ❌ | △ | ❌ | ✅ |
| Language agnostic | ✅ | ✅ | ❌ | ✅ |
| State reconstruction | ❌ | △ | ❌ | 🔜 |
| Time travel | ❌ | ✅ | ❌ | 🔜 |
| AI-agent integration | ❌ | ❌ | ❌ | ✅ |

**rv** makes Ruby fast. **arc** makes *everything* observable.

---

## Architecture: Flux Core

arc is built on **[Flux Core](docs/FLUX_CORE.md)**, a general-purpose event sourcing engine for CLI tools.

```
┌─────────────────────────────────────────────────┐
│                    arc CLI                       │
│  ┌──────┐  ┌──────┐  ┌───────┐  ┌───────────┐  │
│  │ init │  │ exec │  │ state │  │  replay   │  │
│  └──┬───┘  └──┬───┘  └───┬───┘  └─────┬─────┘  │
│     │         │          │             │        │
├─────┴─────────┴──────────┴─────────────┴────────┤
│              Flux Core Engine                    │
│  ┌───────────────┐  ┌──────────────────────┐    │
│  │ FluxProject   │  │  Signal (NDJSON)     │    │
│  │  .init()      │  │  - id (UUID v7)      │    │
│  │  .open()      │  │  - type              │    │
│  │  .record()    │  │  - payload (any JSON) │    │
│  │  .read()      │  │  - timestamp          │    │
│  └───────────────┘  └──────────────────────┘    │
├─────────────────────────────────────────────────┤
│           .flux/signals.jsonl                    │
│         (Append-only NDJSON log)                 │
└─────────────────────────────────────────────────┘
```

Flux Core is designed to be extracted as a standalone crate (`flux-core`) — usable by any Rust CLI tool, not just arc.

---

## Roadmap

See **[ROADMAP.md](ROADMAP.md)** for the full development plan.

| Phase | Status | Description |
|---|---|---|
| **1. Core Engine** | ✅ Done | Signal recording, structured payloads, UUID v7, FluxProject API |
| **2. State Machine** | 🔜 Next | Reconstruct environment state from signal log |
| **3. Replay Engine** | 📋 Planned | Replay signal logs on new machines |
| **4. Ecosystem** | 📋 Planned | crates.io, language bindings, editor extensions |

---

## Philosophy

> **"Tools should remember."**

We live in an era where AI agents execute dozens of commands in seconds.
Shell history isn't enough. Git diffs don't capture the *process*.
arc fills that gap — it gives every terminal operation **identity**, **structure**, and **permanence**.

Not a replacement for your tools. A **memory layer** for all of them.

---

## Tech Stack

- **Language**: Rust (2024 edition)
- **CLI Framework**: clap v4
- **Serialization**: serde + serde_json
- **IDs**: UUID v7 (time-sortable)
- **Log Format**: NDJSON (Newline Delimited JSON)

---

## License

MIT

---

<div align="center">

*Built with the Flux Protocol.*

**arc** — Every operation has meaning.

</div>
