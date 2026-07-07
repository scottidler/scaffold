# Design Document: Reusable-Workflow Callers for Generated CI/Release

**Author:** Scott Idler
**Date:** 2026-06-27
**Status:** Implemented
**Supersedes:** `2026-03-24-github-workflows.md`

## Summary

Replace the fully-inlined `.github/workflows/{ci,release}.yml` that scaffold
generated with **thin caller workflows** that delegate to the reusable workflows
in `<org>/github-actions`:

- `ci.yml` -> `uses: <org>/github-actions/.github/workflows/rust-ci.yml@<ref>`
- `release.yml` -> `uses: <org>/github-actions/.github/workflows/rust-cli-release.yml@<ref>`

The org is resolved per-project (it depends on whether the repo lives under
`scottidler` or `tatari-tv`), so a scaffolded repo calls the reusable workflow in
its *own* org.

## Problem Statement

### Background

The original design (2026-03-24) inlined the entire CI and release pipelines into
every scaffolded repo. That shipped working CI on day one, but every generated
repo carried a ~150-line release matrix and a CI job that drifted independently:
fixing a bug in the pipeline (a stale action SHA, a new toolchain step, the
`otto ci` convergence) meant editing every repo by hand.

Meanwhile `tatari-tv/github-actions` grew two reusable Rust workflows -
`rust-ci.yml` (runs `otto ci`) and `rust-cli-release.yml` (cross-compile + tarball
+ sha256 + GitHub Release) - explicitly designed as "twins" intended to be
identical across the `tatari-tv` and `scottidler` orgs. Scaffolded repos were not
using them.

### Problem

Inlined pipelines can't be fixed centrally. Each scaffolded repo is a fork of the
pipeline frozen at scaffold time. We want new repos to *call* the shared workflow
so pipeline fixes land in one place.

### Goals

- Generate thin caller workflows that `uses:` the org's reusable workflows.
- Resolve the org correctly per-project: a `scottidler` repo calls
  `scottidler/github-actions`; a `tatari-tv` repo calls `tatari-tv/github-actions`.
- Preserve the existing `--rust-version` pin by passing it through as the
  reusable workflow's `rust-version` input.
- Keep zero-config: a freshly scaffolded single-binary crate needs no inputs
  beyond the toolchain.

### Non-Goals

- Authoring the reusable workflows themselves (they already exist; the
  `scottidler/github-actions` twin was created as part of this work by copying the
  tatari-tv files verbatim and tagging `v1`).
- Private-dependency wiring (`private-deps` / `private-deps-token`) - the
  reusable workflows support it, but scaffolded public CLIs don't need it; the
  caller leaves it at its default (`false`).
- Docker/deploy (out of scope for a CLI release, same as the original).

## Proposed Solution

### Org resolution

The reusable repo is always `<org>/github-actions`. `<org>` is resolved in
`main::resolve_workflow_org` with this precedence (matching the project's existing
flag > config > default convention):

1. `--org <org>` CLI flag (explicit override)
2. the owner parsed from the target repo's `git remote get-url origin`
   (scaffold is usually run inside a freshly cloned repo whose `origin` already
   points at the destination org)
3. config `workflow.default-org` (defaults to `scottidler`)

`parse_owner_from_remote` handles the three remote forms scaffold sees:
`git@github.com:owner/repo.git`, `ssh://github.com/owner/repo` (what the `clone`
tool writes), and `https://github.com/owner/repo.git`.

### Ref resolution

The caller pins the reusable workflow to a ref. Precedence:

1. `--workflow-ref <ref>` CLI flag
2. config `workflow.ref` (defaults to `v1`)

`v1` is the scottidler twin's first tag. For a `tatari-tv` repo, pass
`--workflow-ref v5` (or set it in config) to match that org's current tag.

### Generated templates

`ci.yml`:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
permissions:
  contents: read
jobs:
  ci:
    uses: <org>/github-actions/.github/workflows/rust-ci.yml@<ref>
    with:
      rust-version: "<pinned>"
```

`release.yml`:

```yaml
name: Release
on:
  push:
    tags:
      - 'v*'
  workflow_dispatch: {}
permissions:
  contents: write
jobs:
  release:
    uses: <org>/github-actions/.github/workflows/rust-cli-release.yml@<ref>
    with:
      rust-version: "<pinned>"
```

The release caller relies on the reusable workflow defaulting `bin-names` and
`archive-prefix` to the repo name, so a single-binary crate needs no extra inputs.
`contents: write` on the caller is load-bearing: a called workflow's
`GITHUB_TOKEN` permissions are capped by the caller, and publishing the Release
needs write.

### Implementation

- `config.rs`: new `WorkflowConfig { default_org, git_ref }` (`ref` YAML key ->
  `git_ref` field via serde rename), defaulting to `scottidler` / `v1`, added to
  `Config`.
- `cli.rs`: `--org` and `--workflow-ref` flags.
- `main.rs`: `parse_owner_from_remote`, `detect_git_org`, `resolve_workflow_org`;
  org + ref resolved in `create_project` and threaded into `generate_project`.
- `templates.rs`: `generate_github_ci_yml` / `generate_github_release_yml`
  rewritten to emit callers (org/ref/rust-version substituted via `.replace`);
  `generate_github_workflows` and `generate_project` thread `workflow_org` +
  `workflow_ref`. The release generator no longer needs `project_name`.

## Alternatives Considered

### Keep inlining

Rejected - the whole point is central pipeline maintenance. Inlining is what we're
moving away from.

### Hardcode the org to `scottidler`

Rejected - scaffold also serves `tatari-tv` repos, and the user explicitly called
out that the correct org depends on where the repo lives. Auto-detecting from the
git remote makes the common cloned-repo case correct with no flag.

### Single org-agnostic actions repo

Cross-org reusable-workflow calls require the actions repo to be public or to grant
access; a private `tatari-tv/github-actions` can't be called from a personal
`scottidler` repo. Per-org twins sidestep this: each repo calls its own org's copy.

## Technical Considerations

### Security

Callers grant the minimum: `contents: read` for CI, `contents: write` for release
(needed to publish the Release). The reusable workflows pin every third-party
action to a commit SHA. No secrets are required for the default public-CLI path.

### Testing Strategy

- `config.rs`: defaults + `ref` key deserialization.
- `cli.rs`: `--org` / `--workflow-ref` parse and default to `None`.
- `main.rs`: `parse_owner_from_remote` across all three URL forms + non-github
  rejection; `resolve_workflow_org` flag-wins and config-fallback.
- `templates.rs`: generated `ci.yml`/`release.yml` contain the right
  `uses: <org>/github-actions/...@<ref>` line and `rust-version` input, no leftover
  `__ORG__`/`__REF__`/`__RUST_VERSION__` placeholders, and org/ref overrides
  (`tatari-tv`/`v5`) are honored.

## Open Questions

- The scottidler twin starts at `v1` while tatari-tv is at `v5`. If the twins are
  maintained in lockstep going forward, their tags may converge; until then the
  default ref is correct for the common (scottidler) case and `--workflow-ref`
  covers the rest.

## References

- Reusable workflows (tatari-tv): `~/repos/tatari-tv/github-actions/.github/workflows/{rust-ci,rust-cli-release}.yml`
- Reusable workflows (scottidler twin): `github.com/scottidler/github-actions` @ `v1`
- Caller example: `~/repos/tatari-tv/marquee/main/.github/workflows/{ci,release}.yml`
- Scaffold templates: `~/repos/scottidler/scaffold/src/templates.rs`
