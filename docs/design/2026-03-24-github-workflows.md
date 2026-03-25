# Design Document: GitHub Actions Workflow Generation

**Author:** Scott Idler
**Date:** 2026-03-24
**Status:** Implemented
**Review Passes Completed:** 5/5

## Summary

Add GitHub Actions workflow generation to scaffold so that every new Rust CLI project ships with CI and cross-platform binary release workflows out of the box. Templates are modeled on scottidler/otto's workflows - the gold standard for this repo collection.

## Problem Statement

### Background

Scaffold already generates `.otto.yml` for local CI, `clippy.toml`, `build.rs`, and all Rust source files. But when pushing a new project to GitHub, there are no CI or release workflows. Users must manually copy and adapt workflow files from another repo (like otto or aka) every time.

### Problem

Scaffolded projects have no GitHub Actions workflows. Running `git push` to a new repo gives you zero CI and no release pipeline. You end up copying workflows from otto or aka and doing find-replace on the project name - exactly the kind of boilerplate scaffold exists to eliminate.

### Goals

- Generate a CI workflow that runs tests, clippy, and fmt on push/PR
- Generate a release workflow that builds cross-platform binaries (linux-amd64, linux-arm64, macos-x86_64, macos-arm64) and creates a GitHub release on version tags
- Both workflows should work with zero configuration on any Rust CLI project

### Non-Goals

- Docker image building (not every project needs a Dockerfile)
- Custom workflow configuration via scaffold.yml (keep it simple; users edit the generated files)
- Publishing to crates.io
- Setup action generation (otto-specific)

## Proposed Solution

### Overview

Add two new template functions to `src/templates.rs` and a parent function that creates the `.github/workflows/` directory:

1. `generate_github_ci_yml()` - CI workflow
2. `generate_github_release_yml()` - release workflow
3. `generate_github_workflows()` - orchestrator that creates the directory and calls both

### Architecture

The implementation follows the exact same pattern as every other generated file in scaffold:

1. Define content as a raw string literal
2. Use `.replace("{{PROJECT}}", project_name)` for parameterization (avoids escaping `${{ }}` GitHub Actions syntax)
3. Call `write_if_not_exists()` to write the file
4. Wire into `generate_project()` between clippy.toml and .otto.yml generation

### CI Workflow (`.github/workflows/ci.yml`)

Based on otto's `ci.yml` with one change: branch trigger is `[main]` only (otto's `makefile` branch is project-specific).

**Jobs:**
- **test** (ubuntu-latest): install Rust 1.94.0 with rustfmt+clippy, cache deps, run `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`
- **build** (matrix: ubuntu-latest, macos-14): install Rust, cache deps, `cargo build --release`

### Release Workflow (`.github/workflows/release.yml`)

Based on otto's `release-and-publish.yml` with the Docker job removed.

**Trigger:** push tags matching `v*`

**Jobs:**
- **build-linux** (ubuntu-latest, debian:bookworm container): matrix builds for x86_64-unknown-linux-gnu (native) and aarch64-unknown-linux-gnu (cross-compiled with gcc-aarch64-linux-gnu). Produces tarballs + SHA256 checksums.
- **build-macos** (macos-14): matrix builds for x86_64-apple-darwin and aarch64-apple-darwin. Both targets added via rustup. Produces tarballs + SHA256 checksums.
- **create-release** (depends on build-linux + build-macos): downloads all artifacts, creates GitHub release via softprops/action-gh-release@v2.

### Parameterization

The only project-specific value in the workflows is the binary name (used in `cp`, `tar`, artifact names). The approach:

- Use `{{PROJECT}}` as a placeholder in raw string templates
- Replace with `project_name` at generation time
- This avoids needing to escape hundreds of `${{ }}` GitHub Actions expressions that would conflict with Rust's `format!()` macro

### Implementation Plan

**Phase 1** (single commit):
1. Add `generate_github_ci_yml()` to `templates.rs`
2. Add `generate_github_release_yml()` to `templates.rs`
3. Add `generate_github_workflows()` orchestrator
4. Wire into `generate_project()` - call after `generate_clippy_toml()`, before `generate_otto_yml()`
5. Create `.github/workflows/` directory in the orchestrator
6. Add tests for all three functions
7. Update the `test_generate_project_creates_all_files` test to assert workflow files exist

## Alternatives Considered

### Alternative 1: Configurable workflow generation via scaffold.yml

- **Description:** Add `create-github-workflows: true` and `include-docker: false` to scaffold config
- **Pros:** More flexible
- **Cons:** Over-engineering for a feature that should just always be there. Every Rust CLI project on GitHub wants CI and release workflows.
- **Why not chosen:** YAGNI. If someone doesn't want workflows, they can delete the files. Much simpler than configuration.

### Alternative 2: Use aka's simpler release workflow (no linux-arm64)

- **Description:** aka only builds linux-amd64, macos-x86_64, macos-arm64
- **Pros:** Simpler, fewer moving parts
- **Cons:** Missing linux-arm64 which is increasingly important (Graviton, Raspberry Pi, etc.)
- **Why not chosen:** Otto's approach is the gold standard and linux-arm64 is worth having from day one.

### Alternative 3: External template files instead of inline strings

- **Description:** Store workflow templates as separate .yml files and read them at build time via include_str!
- **Pros:** Easier to read and edit the templates
- **Cons:** Breaks the existing pattern where all templates are inline in templates.rs. Would require build.rs changes or a templates/ directory.
- **Why not chosen:** Consistency with existing code. All other templates are inline raw strings.

## Technical Considerations

### Dependencies

No new dependencies. Uses existing `std::fs`, `eyre`, `colored`, and the `write_if_not_exists` helper.

### Performance

Negligible - writing two small YAML files.

### Security

Workflows use `secrets.GITHUB_TOKEN` (built-in) with `contents: write` permission on the release workflow. This is standard and minimal. No third-party actions beyond well-established ones (actions/checkout@v4, dtolnay/rust-toolchain, Swatinem/rust-cache@v2, softprops/action-gh-release@v2).

### Testing Strategy

- Unit tests for each generator function (assert file exists, assert key content strings present)
- Update integration test `test_generate_project_creates_all_files` to check for `.github/workflows/ci.yml` and `.github/workflows/release.yml`
- Manual test: run `scaffold test-project` and verify workflows are valid YAML

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Rust version in template becomes stale | High | Low | It's a starting point; users update as needed |
| GitHub Actions versions become outdated | Medium | Low | Same as above - users update the generated files |
| `write_if_not_exists` + `--force` skips workflows on re-scaffold | Low | Low | Existing behavior - consistent with all other generated files |

## Open Questions

- None - design is straightforward and agreed upon.

## References

- Otto CI workflow: `~/repos/otto-rs/otto/.github/workflows/ci.yml`
- Otto release workflow: `~/repos/otto-rs/otto/.github/workflows/release-and-publish.yml`
- Aka release workflow: `~/repos/scottidler/aka/.github/workflows/binary-release.yml`
- Scaffold templates: `~/repos/scottidler/scaffold/src/templates.rs`
