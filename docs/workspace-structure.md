# Pulse — Workspace Structure

## Overview

Pulse is organized as a Cargo workspace with three primary crates. The workspace ensures `pulse-core` can be compiled independently of Tauri, enabling the CLI to exist as a proper first-class binary and making the core engine testable without any UI overhead.

```
pulse-rs/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── pulse-core/               # platform library (no UI dependencies)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── feeds/            # source adapters + ingestion engine
│   │       ├── storage/          # SQLite layer + migrations
│   │       ├── sync/             # polling scheduler + health tracking
│   │       ├── timeline/         # unified timeline logic + pagination
│   │       ├── search/           # FTS query + ranking
│   │       ├── ai/               # tagging pipeline + model management
│   │       └── config/           # app config + user settings
│   └── pulse-cli/                # CLI binary (depends on pulse-core)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/         # clap subcommand handlers
│           └── tui/              # ratatui interactive interface
├── src-tauri/                    # Tauri shell (depends on pulse-core)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Tauri command wrappers over pulse-core
│       └── main.rs
├── src/                          # SvelteKit frontend
├── docs/                         # architecture documents (this directory)
└── package.json
```

## Workspace Root Cargo.toml

```toml
[workspace]
members = [
    "crates/pulse-core",
    "crates/pulse-cli",
    "src-tauri",
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled-full"] }
uuid = { version = "1", features = ["v5"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
thiserror = "1"
anyhow = "1"
```

Workspace-level dependency declarations avoid version drift across crates. Each crate still opts in explicitly via `{ workspace = true }`.

## Crate Responsibilities

### `pulse-core`

The platform library. Has **zero UI dependencies** — no Tauri, no ratatui, no clap.

Responsibilities:
- Feed source abstraction and adapters (RSS, HN, Reddit)
- Feed item normalization to canonical `FeedItem`
- SQLite storage layer with migrations
- Polling scheduler + backoff + health tracking
- Unified timeline with pagination and filtering
- FTS5 search
- AI tagging pipeline (rule-based Phase 1, ONNX Phase 4)
- User configuration persistence

This crate compiles to:
- `rlib` — used by `pulse-cli` and `src-tauri`
- In the future: `cdylib` for Android JNI bindings if needed

`pulse-core` must be usable with **no Tokio runtime** (for sync contexts) and **with Tokio** (for async contexts). Async functions use `async fn`; blocking SQLite calls run on `tokio::task::spawn_blocking`.

### `pulse-cli`

The CLI binary. Phase 1's primary interface.

Responsibilities:
- `clap`-based argument parsing
- Subcommand dispatch to `pulse-core` APIs
- `ratatui` TUI for interactive timeline browsing
- Output formatting (human-readable + `--json`)
- Progress indicators for long sync operations

The CLI does **no business logic** — it only translates CLI invocations into `pulse-core` calls and formats the results.

### `src-tauri`

The Tauri application shell. Deferred to Phase 3.

Responsibilities:
- Tauri command handlers (`#[tauri::command]`) wrapping `pulse-core` APIs
- Background task management (Tauri's task API)
- Android-specific storage path resolution
- Permission/capability declarations

The Tauri crate is also logic-free. It delegates everything to `pulse-core`.

## Feature Flags

`pulse-core` exposes feature flags to control optional compilation:

```toml
[features]
default = ["sync", "search"]
sync = []          # polling scheduler (can disable for pure-offline builds)
search = []        # FTS5 indexing (requires bundled-full SQLite)
ai-rules = []      # deterministic rule-based tagging (Phase 1)
ai-onnx = []       # ONNX model inference (Phase 4, pulls in ort crate)
```

`ai-onnx` is NOT in `default`. Enabling it pulls in the `ort` crate (ONNX Runtime bindings), which is large (~10MB compiled). Users must explicitly opt in to model-backed inference.

## Test Organization

```
crates/pulse-core/
└── src/
    ├── feeds/
    │   ├── mod.rs
    │   └── tests.rs          # unit tests for normalization logic
    ├── storage/
    │   ├── mod.rs
    │   └── tests.rs          # integration tests against in-memory SQLite
    └── ...

crates/pulse-core/tests/       # integration tests (full pipeline)
    ├── ingestion_test.rs
    ├── timeline_test.rs
    └── search_test.rs

crates/pulse-cli/tests/        # CLI output format tests
    └── commands_test.rs
```

Unit tests use in-memory SQLite (`:memory:`) so they are fast and have no filesystem side effects. Integration tests use a temp directory with real SQLite files.

## Build Targets Summary

| Target | Command | Output |
|---|---|---|
| CLI binary | `cargo build -p pulse-cli` | `pulse` binary |
| Core library | `cargo build -p pulse-core` | `libpulse_core.rlib` |
| Tauri desktop | `pnpm tauri build` | desktop app |
| Tauri Android | `pnpm tauri android build` | APK |
| Tests | `cargo test -p pulse-core` | test runner |
| Benchmarks | `cargo bench -p pulse-core` | criterion reports |

## Rationale

**Why not a single binary with feature flags instead of a workspace?**
A workspace makes `pulse-core` independently versioned and testable. It also makes the boundary between "platform logic" and "UI glue" structurally enforced — you cannot accidentally import Tauri into the core engine if the Tauri crate is a separate workspace member.

**Why not put CLI code into src-tauri?**
`src-tauri` requires the Tauri build system. Building the CLI binary for development iteration should not require the Tauri toolchain. Separation means `cargo run -p pulse-cli` works without `pnpm`, Tauri CLI, or Android SDK.

**Why keep src-tauri at the root rather than under crates/?**
Tauri's build system (`tauri.conf.json`, Android generation) assumes a specific directory structure relative to the package root. Moving it would require significant reconfiguration of generated Android files.
