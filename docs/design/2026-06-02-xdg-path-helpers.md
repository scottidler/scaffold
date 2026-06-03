# XDG-correct path helpers in scaffolded projects

> **Status: IMPLEMENTED.** Decision taken: **Full-XDG** (honor `$XDG_CONFIG_HOME` /
> `$XDG_DATA_HOME`, then `$HOME` fallback), mirroring `tatari-tv/pagerduty-cli`.
> The scaffold now generates `xdg_config_dir()` / `xdg_data_dir()` into the project's
> `config.rs`, uses them in the generated `main.rs` (logs) and `config.rs` (config), and
> emits env-honoring tests (no macOS `Application Support` assertion). A generated
> `cli.rs` `after_help` advertises the XDG log path. The `scaffold` tool itself dogfoods
> the same helpers. The global `rust.md` rule was reconciled (the "When in Rome - use
> `dirs`" section is gone; it now mandates the XDG helpers) and the project memory updated.

Pickup doc. Goal: make scaffolded CLIs resolve config and data/log paths to the
XDG layout on **every** platform, instead of relying on `dirs`, which is only
XDG-correct on Linux.

## The problem

- `dirs::config_dir()` and `dirs::data_local_dir()` honor `$XDG_CONFIG_HOME` /
  `$XDG_DATA_HOME` **only on Linux**.
- On macOS they resolve via system APIs and return `~/Library/Application Support`,
  ignoring the env vars.
- Net effect: a scaffolded tool tells the user (in `--help`) that logs live at
  `~/.local/share/<proj>/logs/...`, but on a Mac they actually land in
  `~/Library/Application Support/<proj>/logs/...`. The advertised path is a lie on mac.
- This bit a real user: `tatari-tv/pagerduty-cli` logs went to
  `~/Library/Application Support/pagerduty-cli/logs/` on macOS.

## Current scaffold state

- `src/templates.rs` generates `dirs::config_dir()` (config) and
  `dirs::data_local_dir()` (logs). Commit `37c8591` (2026-04-28) standardized on
  this and added platform-path tests that **assert** the macOS
  `~/Library/Application Support` behavior.
- So the scaffold default is mac-native, NOT XDG. This matches the global
  `rust.md` "when in Rome, use dirs" rule, and contradicts the `~/.local/share`
  string the templates print in `--help`.

## Reference implementation (already shipped)

`tatari-tv/pagerduty-cli` now does the fully-correct thing. Lift this pattern:

- `src/config.rs` `xdg_config_dir()`: honor `$XDG_CONFIG_HOME` if absolute, else
  `dirs::home_dir().join(".config")`.
- `src/config.rs` `xdg_data_dir()`: honor `$XDG_DATA_HOME` if absolute, else
  `dirs::home_dir().join(".local").join("share")`. Marked `pub` so `main.rs` can use it.
- `src/main.rs` `setup_tracing()`: build `log_dir` from
  `xdg_data_dir().join("<proj>").join("logs")`, NOT `dirs::data_local_dir()`.
- Test: `config::tests::test_xdg_data_dir_honors_env_and_falls_back` (sets/unsets
  `$XDG_DATA_HOME` under an `ENV_LOCK` mutex, asserts env-honoring + `.local/share`
  fallback).

## Work to do in scaffold

- Generate `xdg_config_dir()` and `xdg_data_dir()` into the config template.
- Replace template uses of `dirs::config_dir()` -> `xdg_config_dir()` and
  `dirs::data_local_dir()` -> `xdg_data_dir()` (config path + log/data path).
- Rewrite the generated platform-path tests: stop asserting
  `~/Library/Application Support` on macOS; assert the XDG layout on both platforms
  (env-var-honoring + `$HOME/.local/share` fallback). Mirror the pagerduty-cli test.
- Update the `templates.rs` self-tests that currently assert the generated source
  contains `dirs::data_local_dir()` / `dirs::config_dir()` (around the
  `main_rs.contains(...)` / `config_rs.contains(...)` assertions).
- The generated `after_help` log-path string (`~/.local/share/<proj>/logs/...`)
  needs no change; the helper finally makes it true on mac.

## Open decision before starting

- **Full-XDG** (honor `$XDG_DATA_HOME` / `$XDG_CONFIG_HOME`, then `$HOME` fallback):
  what pagerduty-cli does. Recommended: more correct, env-overridable, symmetric.
- **Hardcoded `$HOME/.local/share`** (no env var): simpler, used by `scottidler/aka`
  (`src/lib.rs`) and `scottidler/manifest` (`src/main.rs`), both done July 2025.
- Whichever wins, reconcile the global `rust.md` rule (which currently says "use
  `dirs`, do not override the macOS path") so the scaffold and the rule agree.

## Prior art

- `scottidler/aka`, `scottidler/manifest`: hardcoded `$HOME/.local/share` log paths (July 2025).
- `tatari-tv/pagerduty-cli`: full-XDG `xdg_config_dir()` + `xdg_data_dir()` (June 2026).
- Gotcha documented in `scottidler/second-brain` `cortex/src/sweep.rs` (macOS ignores `$XDG_DATA_HOME`).
