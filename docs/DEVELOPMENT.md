# Development Guide

This guide is a fast reference for day-to-day DokTUI development. For the full project handbook, see [`PROJECT-GUIDE.md`](PROJECT-GUIDE.md).

## Prerequisites

- A recent stable Rust toolchain ([rustup.rs](https://rustup.rs))
- Git
- (Optional) [`just`](https://github.com/casey/just) — a command runner. If you do not have it, run the equivalent `cargo` commands directly.

## Quick commands

We provide a `justfile` at the repo root. Run `just` to see all recipes.

| Recipe | What it does |
|---|---|
| `just dev` | Build and run the app (`cargo run`) |
| `just test` | Run all tests (`cargo test --all-targets`) |
| `just check` | Fast type check (`cargo check --all-targets`) |
| `just fmt` | Format all Rust code (`cargo fmt`) |
| `just clippy` | Run Clippy (`cargo clippy --all-targets`) |
| `just verify` | Run `clippy`, `check`, and `test` in one go |
| `just lint` | Run `fmt`, `clippy`, `check`, and `test` in one go |
| `just release` | Build an optimized binary (`cargo build --release`) |

Always run `just verify` before opening a PR.

## Running with logging

DokTUI uses `tracing`. To see debug logs while running:

```sh
RUST_LOG=debug cargo run
```

Use `RUST_LOG=doktui=trace` for more noise from the app itself.

## Project structure

| Path | Responsibility |
|---|---|
| `src/app/` | The Elm Architecture core: `state`, `event`, `command`, `mod` (loop + `update`) |
| `src/ui/` | Everything visual: `views/`, `editor/`, `layout/`, `theme/`, `components.rs`, `sprite.rs`, `anim.rs` |
| `src/services/` | SSH, Docker, Traefik, routing, provisioning, secrets, cron, updater |
| `src/config/` | Config loading/saving and OS-specific paths |
| `src/security/` | Dedicated SSH key, keychain, host-key verification |
| `src/i18n/` | Fluent helpers and bundled locale loading |
| `themes/` | Bundled TOML themes |
| `locales/` | Fluent `.ftl` translations |
| `docs/` | Product/design/technical documentation |

## Architecture in one paragraph

DokTUI follows **The Elm Architecture (TEA)**:

1. Keyboard/mouse/SSH events become a `Message`.
2. `map_key` converts keys into `Message`s.
3. The `update` function processes `Message`s and mutates `AppState`.
4. The UI re-renders from `AppState`.
5. I/O runs in background `tokio` tasks and sends results back into the `update` loop via `mpsc`.

## Common tasks

### Add a new screen

1. Add a `Screen` variant in `src/app/state.rs`.
2. Add `Message` variants in `src/app/event.rs`.
3. Handle keyboard input in `map_key` (`src/app/mod.rs`).
4. Handle the messages in `update` (`src/app/mod.rs`).
5. Create a view file in `src/ui/views/<screen>.rs`.
6. Wire the render call in `src/ui/mod.rs`.
7. Add i18n keys to `locales/en.ftl`.
8. Update the `shortcut_line` at the bottom of the view.
9. If the screen introduces new global keys, update `README.md` §Keyboard & Mouse.

### Add a new keybinding

1. Add the mapping in `src/app/mod.rs` `map_key`.
2. Add the handler in `src/app/mod.rs` `update` if the message is new.
3. Update the view’s `shortcut_line` in `src/ui/views/<screen>.rs`.
4. Add or reuse i18n keys in `locales/en.ftl`.
5. Update `README.md` keyboard table if the key is global or user-facing.

### Add a new theme

1. Copy `themes/gruvbox-material.toml` or `themes/pico8.toml`.
2. Edit colors and glyphs.
3. Register it in `src/ui/theme/registry.rs` `BUILTIN`.
4. Run `cargo test` to ensure the theme resolves.

### Add a new translation

1. Copy `locales/en.ftl` to `locales/<lang>.ftl`.
2. Translate strings.
3. Set `locale = "<lang>"` in `~/.config/doktui/config.toml` or run `doktui --locale <lang>`.

### Add i18n strings

1. Add the key to `locales/en.ftl`.
2. Use `state.i18n.t("my-key")` or `state.i18n.t_fmt("my-key", &[("name", &value)])`.
3. Keep placeholders consistent with the Fluent definition.

## Testing

- Unit tests are near the code they test, under `#[cfg(test)]` modules.
- Run the whole suite with `just test`.
- For service logic, prefer tests that do not require a real SSH server (mock inputs or pure functions).
- UI rendering code is mostly tested by `cargo check`; unit tests for `map_key` live in `src/app/mod.rs`.

## Code style

- Run `cargo fmt` before committing.
- Run `cargo clippy --all-targets` and fix warnings.
- Do not hardcode colors; use `theme.color(Role::…)` and `theme.style(Role::…)`.
- Do not put I/O in view code; keep rendering pure from `AppState`.

## Troubleshooting

### `cargo build` fails on Windows

- Make sure you have the MSVC toolchain or the `stable-x86_64-pc-windows-gnu` toolchain.
- OpenSSL/Russh can sometimes require `vcpkg` on Windows; the project uses `russh` with its default Rust crypto.

### SSH connection never connects

- Check that the dedicated SSH key in the OS data dir is added to `~/.ssh/authorized_keys` on the server.
- Run with `RUST_LOG=debug` to see the SSH handshake.

### Theme does not appear

- Verify it is in `src/ui/theme/registry.rs` `BUILTIN` and passes validation in `src/ui/theme/validate.rs`.

## Verification before PR

```sh
just verify
```

`just verify` runs `cargo clippy`, `cargo check`, and `cargo test`. Run `just lint` if you also want `cargo fmt` to format the code first.

## See also

- [`PROJECT-GUIDE.md`](PROJECT-GUIDE.md) — full project handbook
- [`TDD.md`](TDD.md) — technical design document
- [`DESIGN.md`](DESIGN.md) — visual and theme system design
- [`TESTING.md`](TESTING.md) — testing locally without a remote VPS
- [`AGENTS.md`](../AGENTS.md) — agent verification commands
