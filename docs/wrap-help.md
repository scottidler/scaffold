# clap `wrap_help` is a scaffold default

Every Rust CLI `scaffold` generates now enables clap's `wrap_help` feature. This
is the default going forward. Proven on `bump` v0.3.1.

## Why

Default clap does not wrap long help text to the terminal. A multi-sentence
`about` or a long subcommand description prints as one unbroken line that either
runs off the right edge or gets hard-wrapped by the terminal with no indent, so
the continuation lines jam against the left margin and the help becomes a wall.

`wrap_help` makes clap detect the real terminal width (via `terminal_size`) and
wrap each description with a hanging indent aligned under its column, reflowing
when the window resizes. When stdout is not a tty (piped), it falls back to a
fixed width instead of not wrapping at all.

### Before (default clap)

```
Commands:
  release  Release the current repo: on ungated main this lands a version commit, pushes it, then tags and pushes the tag; on a gated repo it rides the bump ...
```

### After (`wrap_help`)

```
Commands:
  release  Release the current repo: on ungated main this lands a version commit, pushes it, then
           tags and pushes the tag; on a gated repo it rides the bump on a feature branch, pushes
           it, opens a PR if none is open, and pauses for `bump finish` after merge
```

## The change

Generated `Cargo.toml`:

```toml
clap = { version = "4", features = ["derive", "wrap_help"] }
```

In the scaffold itself: `src/config.rs`, the clap `Dependency` carries
`features: ["derive", "wrap_help"]`; the test
`test_template_config_clap_has_derive_and_wrap_help_features` asserts both.

## Cost

Pulls in `terminal_size` (tiny). No API or code change in generated projects --
it is purely a help-rendering improvement. Zero downside; it is on by default.
