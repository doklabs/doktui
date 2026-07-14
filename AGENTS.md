# Agent Notes

## Verification

Run these before considering a change complete:

- `cargo check --all-targets`
- `cargo test --all-targets`

## Build

- `cargo build`

## Theme / UI

- Themes live in `themes/*.toml` and are embedded in `src/ui/theme/registry.rs`.
- Add new themes to `BUILTIN` in `src/ui/theme/registry.rs`.
- `Theme` model is in `src/ui/theme/model.rs` with `resolve.rs`/`validate.rs`.
- CLI theme override: `--theme <name>` and `doktui themes list`.
