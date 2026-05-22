# Design Document: Lib+Bin as Default Scaffold Shape

**Author:** Scott Idler
**Date:** 2026-05-22
**Status:** Implemented
**Review Passes Completed:** 5/5
**Shipped in:** v0.1.12

## Summary

Reshape scaffold's output so every generated Rust CLI project ships as both a library (`src/lib.rs`) and a binary (`src/main.rs`) by default. The library owns all business logic; the binary is a thin clap+logging shell that delegates into the library. This aligns scaffold with the convention documented in `rules/rust.md` ("main.rs is a thin shell, lib.rs holds all business logic") and makes every scaffolded CLI immediately consumable as a library by other crates.

## Problem Statement

### Background

Scaffold today produces a **functionally binary-only** crate. The generated tree looks like this:

```
src/
├── cli.rs        # pub struct Cli (clap derive)
├── config.rs     # pub struct Config + load()
├── config/tests.rs
└── main.rs       # mod cli; mod config; run_application(); main()
```

The generated `main.rs` declares `mod cli; mod config;` and contains all business logic. There is no generated `src/lib.rs`. Consumers that try `cargo add <generated-project>` get a usable library crate only if Cargo's defaults happen to expose one - and since the package has no `lib.rs`, they get nothing usable. The clap struct, the config loader, and the run logic all live as submodules of the binary crate.

(Note: scaffold's *own* repo has a vestigial `src/lib.rs` declaring `pub mod cli; pub mod config; pub mod templates;` that duplicates `main.rs`'s `mod cli; mod config; mod templates;`. This is a separate cleanup tracked in Future Work and does **not** describe what scaffold *generates*.)

### Problem

Two concrete failure modes:

1. **Can't reuse as a library.** When a scaffolded CLI's logic becomes useful to another tool (e.g., `borg` wanting to import `signal-rs`'s send/receive operations), the first step is always "reshape the existing CLI into lib+bin." That reshape is mechanical, repetitive, and consistently regrettable - it should have been the starting shape.

2. **Inconsistent with its own conventions.** `rules/rust.md` § "Architecture: Shell/Core Split" prescribes exactly the lib+bin pattern that scaffold currently fails to emit. The tool whose purpose is "embody my conventions for new CLIs" is the loudest violator of those conventions.

### Goals

- Every newly scaffolded project includes both `src/lib.rs` (the meat) and `src/main.rs` (the shell), and builds cleanly with `cargo build` and `cargo build --lib`.
- The generated `main.rs` is a thin shell: setup logging, parse `Cli`, load `Config`, call `lib::run(&config)`, exit. No business logic.
- The generated `lib.rs` exposes `pub fn run(config: &Config) -> Result<RunResult>` as the entry point and re-exports `Config`. It does not depend on `clap`.
- The reshape is the **only** shape - no opt-out flag.
- Hyphenated project names (`signal-rs`) Just Work; the binary references the library by its hyphen-translated crate name (`signal_rs`).

### Non-Goals

- **Cleaning up scaffold's own vestigial `src/lib.rs`.** Scaffold's repo has both `src/main.rs` (with `mod cli; mod config; mod templates;`) and a `src/lib.rs` that re-declares the same modules with `pub mod`. The duplicate declarations cause the module tree to compile twice. This cleanup is tracked in Future Work; it neither blocks nor is blocked by this design.
- **Cli → Config translation via `TryFrom<Cli>`.** The convention recommends it, but the existing scaffold's `Config::load(Option<&PathBuf>)` flow is independent of `Cli` already. Leave that as it is; introducing TryFrom is a separate ergonomic refactor.
- **Removing the `--verbose` flag** from generated CLIs. The stub keeps it but stops threading it into the library (see Architecture).
- **Adding integration tests under `tests/`.** The lib gets its own unit tests in `src/tests.rs` (the crate-root submodule convention for `src/lib.rs`); no new top-level test directory.
- **Opt-out flag for binary-only output.** Considered and rejected (see Alternatives).

## Proposed Solution

### Overview

Add a `generate_lib_rs` template that emits `src/lib.rs` and `src/tests.rs`, rewrite `generate_main_rs` to emit a thin shell, and wire both into `generate_project`. No changes to the generated `Cargo.toml` - Cargo's default file-based crate detection handles `lib.rs` + `main.rs` correctly.

### Architecture

```
src/
├── cli.rs            # unchanged content; declared from main.rs (binary-side)
├── config.rs         # unchanged content; declared from lib.rs (library-side)
├── config/tests.rs   # unchanged
├── lib.rs            # NEW: pub mod config + pub fn run() + RunResult
├── tests.rs          # NEW: smoke test for run() (crate-root submodule of lib.rs)
└── main.rs           # REWRITTEN: shell only (logging + parse + call + exit)
```

Per the Rust 2018+ submodule convention, the crate root `lib.rs` declares `#[cfg(test)] mod tests;` and its bodies live in `src/tests.rs` (not `src/lib/tests.rs`). `cli.rs` stays where it is on disk; it is declared from `main.rs` (`mod cli;`) and thus compiles as part of the binary crate, keeping `clap` out of the library's dependency tree.

**Module ownership at a glance** - this is the only non-obvious thing about the shape:

| File              | Declared from         | Crate                       |
|-------------------|-----------------------|-----------------------------|
| `cli.rs`          | `main.rs` (`mod cli`) | binary                      |
| `config.rs`       | `lib.rs` (`pub mod config`) | library                |
| `tests.rs`        | `lib.rs` (`mod tests`, test-only) | library                |
| `config/tests.rs` | `config.rs` (`mod tests`, test-only) | library             |

`Cargo.toml` is **unchanged** by this design. Cargo's default file-based crate detection sees both `src/lib.rs` and `src/main.rs` and builds both crates; the library inherits the package name with hyphens translated to underscores, the binary inherits the package name verbatim.

Cargo's defaults give us:
- Library crate name = package name with `-` → `_` (e.g., `signal-rs` → `signal_rs`)
- Binary crate name = package name (e.g., `signal-rs`)
- No `[lib]` or `[[bin]]` section needed in `Cargo.toml`

The generated `main.rs` imports the library by its hyphen-translated name:

```rust
use signal_rs::{Config, run};
```

The `Cli` struct stays as a submodule of the **binary** crate (`mod cli;` in `main.rs`), not the library. This keeps `clap` types out of the library's **API surface** - consumers of the library see only `Config`, `RunResult`, and `run()`, none of which mention `clap`. (Note: `clap` remains in the package's `[dependencies]` so downstream consumers do still resolve it during dependency resolution; that is unavoidable in a single-package lib+bin layout and is the same trade-off ripgrep, fd, and bat accept. The win is API hygiene, not dep-graph reduction.)

### Data Model

One new public type in `lib.rs`:

```rust
pub struct RunResult {
    pub messages: Vec<String>,
}
```

The stub's `run()` returns a small set of pre-formatted (colored) status messages that `main.rs` prints to stdout. When the user replaces stub logic with real work, `RunResult` becomes the natural place to add counts, paths, or summary fields without changing the call site.

### API Design

#### Generated `src/lib.rs`

```rust
#![deny(clippy::unwrap_used)]
#![deny(dead_code)]
#![deny(unused_variables)]

pub mod config;

pub use config::Config;

use colored::*;
use eyre::Result;
use log::info;

#[derive(Debug)]
pub struct RunResult {
    pub messages: Vec<String>,
}

pub fn run(config: &Config) -> Result<RunResult> {
    info!("Running with config: name={} age={}", config.name, config.age);

    let messages = vec![
        format!("{} Configuration loaded successfully", "✓".green()),
        format!("{} Hello from {}!", "🎉".green(), env!("CARGO_PKG_NAME").cyan()),
        format!("{} Author: {}", "👤".blue(), config.name),
        format!("{} Age: {}", "📅".blue(), config.age),
    ];

    Ok(RunResult { messages })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
```

The `#[allow(clippy::unwrap_used)]` on the `mod tests;` declaration is required because the crate root denies that lint and the test bodies use `.unwrap()` freely. This mirrors the existing convention in scaffold's own `src/main.rs:286-287`.

#### Generated `src/tests.rs`

```rust
use super::*;

#[test]
fn test_run_returns_messages() {
    let config = Config::default();
    let result = run(&config).unwrap();
    assert!(!result.messages.is_empty());
}

#[test]
fn test_run_messages_mention_project_name() {
    let config = Config::default();
    let result = run(&config).unwrap();
    let joined = result.messages.join("\n");
    assert!(joined.contains(env!("CARGO_PKG_NAME")));
}
```

#### Generated `src/main.rs`

```rust
#![deny(clippy::unwrap_used)]
#![deny(dead_code)]
#![deny(unused_variables)]

use clap::Parser;
use colored::*;
use eyre::{Context, Result};
use log::info;
use std::fs;
use std::path::PathBuf;

use {{crate_name}}::{Config, run};

mod cli;
use cli::Cli;

fn setup_logging() -> Result<()> {
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("{{project_name}}")
        .join("logs");

    fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    let log_file = log_dir.join("{{project_name}}.log");

    let target = Box::new(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .context("Failed to open log file")?,
    );

    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .init();

    info!("Logging initialized, writing to: {}", log_file.display());
    Ok(())
}

fn main() -> Result<()> {
    setup_logging().context("Failed to setup logging")?;

    let cli = Cli::parse();

    if cli.verbose {
        println!("{}", "🔍 Verbose mode enabled".yellow());
    }

    let config = Config::load(cli.config.as_ref()).context("Failed to load configuration")?;

    info!("Starting with config from: {:?}", cli.config);

    let result = run(&config).context("Application failed")?;
    for message in result.messages {
        println!("{}", message);
    }

    Ok(())
}
```

`{{crate_name}}` is `{{project_name}}.replace('-', '_')`. `{{project_name}}` is the original (possibly hyphenated) name. The `--verbose` flag stays on `Cli` and is read here in the binary; it is *not* passed into the library (terminal decoration is a shell concern).

### Implementation Plan

#### Phase 1: Template changes
**Model:** sonnet

- Add `generate_lib_rs(src_dir, force)` to `src/templates.rs`. Emits `src/lib.rs` with the body above. The `pub use config::Config;` line and the `RunResult` struct are hardcoded; no per-project substitution needed.
- Add `generate_lib_tests_rs(src_dir, force)` to `src/templates.rs`. Emits `src/tests.rs` (sibling of `src/lib.rs`, per the Rust 2018+ submodule convention for crate-root tests) with the two smoke tests.
- Rewrite `generate_main_rs` in `src/templates.rs` to emit the thin-shell form. Compute `crate_name = project_name.replace('-', '_')` inline and inject it into the `use` statement. `mod cli;` stays in the generated `main.rs`; the `mod config;` declaration moves to `lib.rs`.
- Tighten the project-name validation in `src/main.rs:103` (currently `chars().all(alphanumeric | _ | -)`) to also reject names whose first character is an ASCII digit. Without this, a name like `1pass` passes validation today, and the new `crate_name = "1pass".replace('-', '_')` would emit `use 1pass::{Config, run};` in the generated `main.rs` - a Rust syntax error. Rejecting at validation time surfaces the problem to the user with a clear error rather than producing a broken generated project. (Cargo itself warns on leading-digit package names, so this aligns scaffold's contract with Cargo's expectations.)
- Wire `generate_lib_rs` and `generate_lib_tests_rs` into `generate_project` (the orchestrator at `src/templates.rs:20`). They run between `generate_build_rs` and `generate_main_rs` so that the library exists before the binary template references it; the existing `generate_cli_rs` / `generate_config_rs` / `generate_config_tests_rs` calls move unchanged.

#### Phase 2: Scaffold self-tests
**Model:** sonnet

- Add assertions to existing `test_create_project_*` tests in `src/main.rs` that verify `src/lib.rs` and `src/tests.rs` exist after scaffolding.
- Add a new test `test_generated_main_uses_library` that scaffolds a project and asserts the emitted `main.rs` contains a `use <crate_name>::` line referencing the library and does **not** contain a `mod config;` declaration (which has moved to `lib.rs`). The emitted `main.rs` still contains `mod cli;`.
- Add a new test `test_generated_main_translates_hyphens` that scaffolds `foo-bar`, reads the emitted `main.rs`, and asserts the `use` statement reads `use foo_bar::` (not `use foo-bar::` which wouldn't compile).

#### Phase 3: Smoke verification
**Model:** sonnet

- Scaffold a hyphenated project (`scaffold-test-rs`) into a tempdir.
- Run `cargo build` (both crates), `cargo build --lib`, `cargo test` against the generated project.
- All three must succeed. This catches any wiring errors that unit tests can't (hyphen-translation, default-cargo-rules behavior, lib visibility of `Config`).

## Alternatives Considered

### Alternative 1: Opt-in flag (`--lib` or `--with-lib`)

- **Description:** Default to the current binary-only shape; add a `--lib` flag that triggers lib+bin emission.
- **Pros:** Backwards-compatible with existing scaffold muscle memory. Lets project-specific judgment decide.
- **Cons:** Every flag is a permanent maintenance tax (two template paths to keep in sync, two test surfaces, one more decision per `scaffold` invocation). The whole value proposition of scaffold is "embody the convention" - a flag means "follow the convention sometimes."
- **Why not chosen:** The asymmetry strongly favors default-lib. If a particular CLI really doesn't want a library, deleting `src/lib.rs` and inlining `run()` into `main.rs` is a 2-minute manual step. The reverse (need a library later, must reshape) is the exact pain this design exists to eliminate.

### Alternative 2: Keep `Cli` in the library (re-exported from `lib.rs`)

- **Description:** `pub mod cli;` lives in `lib.rs` alongside `pub mod config;`, and `lib.rs` re-exports `pub use cli::Cli;`. The lib's `run()` takes `(cli: &Cli, config: &Config)`.
- **Pros:** Slightly less code in `main.rs`. Easier for downstream library consumers to construct a `Cli` programmatically if they ever want to.
- **Cons:** The library's public API surface includes a `clap::Parser`-derived struct, which couples the domain library to a CLI-parsing concern. Downstream library consumers would need to know `clap` to construct a `Cli` value, and any change to the binary's flags becomes a library API change.
- **Why not chosen:** Violates the rust.md guidance ("test Config validation, not clap parsing"). The library should expose typed domain inputs (`Config`), not the raw shell-flavor input (`Cli`).

### Alternative 3: `run()` returns `Result<()>` with `println!` inside

- **Description:** Skip `RunResult`. Let `lib::run()` `println!` directly. Same as today's `run_application`, just relocated.
- **Pros:** Simplest possible signature. One fewer type.
- **Cons:** Violates rust.md ("never print to stdout/stderr from core"). When the user replaces stub logic with real logic that produces structured output (a list of files processed, a count, a status), they have to refactor the signature **and** the call site. Pre-paying for `RunResult` makes the convention obvious from the first commit.
- **Why not chosen:** The whole point of the reshape is to model the convention well. Cutting this corner undermines that.

## Technical Considerations

### Dependencies

No new dependencies. The library uses `colored`, `eyre`, `log`, and `serde`/`serde_yaml` (via `Config`) - all of which the binary already pulls in. The library does **not** depend on `clap`.

### Performance

A single-package lib+bin build compiles the library first, then the binary against it. For a fresh-scaffold-sized crate, the overhead is on the order of 50-100 ms - imperceptible in practice and outweighed by the test ergonomics this unlocks.

### Security

No security implications. The change is purely structural.

### Testing Strategy

Three layers:

1. **Generated-project unit tests** (live inside the scaffolded crate): `src/tests.rs` exercises `run()` directly using `Config::default()`. `src/config/tests.rs` continues to exercise `Config::load` and platform paths. The lib being unit-testable is the whole point of the reshape.

2. **Scaffold self-tests** (live in scaffold's own `src/main.rs` test module): assert that generated projects have `src/lib.rs`, that the generated `main.rs` references the library by its hyphen-translated name, and that no business logic remains in the generated `main.rs`.

3. **End-to-end smoke** (manual via the smoke script in Phase 3): scaffold a hyphenated project into a tempdir and run `cargo build`, `cargo build --lib`, `cargo test` against it. This catches mistakes the other two layers can't see (default-cargo-rules wiring, visibility, hyphen-translation in the `use` statement).

### Rollout Plan

Standard scaffold release:

1. Merge to `main`.
2. `bump` patch (no API break for existing scaffolded projects - they're untouched; only new projects get the new shape).
3. `cargo install --path .`.
4. Manually scaffold the next real project (the signal-rs reshape this came from) and verify the shape lands clean.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Hyphen-translation bug in `use` statement makes generated `main.rs` not compile | Medium | High | Phase 2 adds an explicit test (`test_generated_main_translates_hyphens`); Phase 3 smoke-builds a hyphenated project |
| Generated `lib.rs` is missing a `pub use` that consumers expect (e.g., `RunResult`) | Low | Medium | Phase 2's `test_generated_main_uses_library` smoke-builds a project, surfacing missing exports as compile errors |
| `Config` already lives in the binary's module tree, and moving its `mod` declaration from `main.rs` to `lib.rs` breaks something subtle | Low | Medium | The submodule files (`config.rs`, `config/tests.rs`) are unchanged - only the parent module owning them changes. Phase 3's smoke build catches any breakage |
| Stub `RunResult` design proves clumsy when user wires real logic | Medium | Low | `RunResult` is a one-field struct (`messages: Vec<String>`); evolving it is a non-breaking change for stub-stage code. Even a full redesign costs five minutes |
| Downstream consumer of the library is surprised that `clap` is in their resolved dep graph | Low | Low | Documented in Architecture: single-package lib+bin shares `[dependencies]` across crate-targets. API hygiene (no `clap` types in the lib's API) is the win, not dep-graph reduction. Workspace split is the escape hatch if a consumer ever needs it |

## Open Questions

- [ ] (Resolved during pass 2.) ~~Should `lib.rs` `pub use RunResult`?~~ Not needed: `RunResult` is defined directly in `lib.rs` (the crate root), so it is already reachable as `<crate>::RunResult` for any consumer.
- [ ] (Resolved during pass 2.) ~~Should we drop `--verbose` from the generated `Cli`?~~ Keep it. The flag now lives on `Cli` (binary-side), is read in `main.rs` for terminal decoration, and is intentionally not passed into the library. If a future user wants verbose to control log level, that is a `--log-level` follow-up (per `rules/rust.md` § Logging, which prefers an explicit log-level flag over RUST_LOG anyway).

## Future Work

These are explicit follow-ups, sequenced after this design ships:

1. **Remove scaffold's vestigial duplicate-module declarations.** Scaffold's repo currently has both `src/main.rs` and a `src/lib.rs` that declare the same module tree (`cli`, `config`, `templates`), causing those modules to compile twice. Bring scaffold itself into the same lib+bin shape this design emits: `lib.rs` owns the modules, `main.rs` drops its duplicate `mod` declarations and consumes the lib by name. Mechanical change; same pattern as the templates this design produces.
2. **Replace `--verbose` with `--log-level` in the generated `Cli` template.** The rust.md rules call for an explicit log-level flag (and forbid `RUST_LOG`), but the existing template still ships `--verbose`. Worth a dedicated design once the lib+bin reshape has landed.
3. **Two-stage `Cli → Config` via `TryFrom`.** The rust.md rules describe a `Cli` (parsing) → `Config` (validation) split via `TryFrom<Cli>`. The current scaffold's `Config::load(Option<&PathBuf>)` is independent of `Cli`; folding the rest of the `Cli`-derived state into `Config::try_from(cli)` would close the convention loop.

## References

- `rules/rust.md` § "Architecture: Shell/Core Split" - the convention this design enforces
- `docs/design/2026-03-24-github-workflows.md` - prior scaffold template-additions design (same pattern)
- Skill: `~/.claude/skills/rust-cli-coder/SKILL.md` - parallel reference for the shell+core split
