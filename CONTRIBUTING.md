# Contributing to DokTUI

Thanks for helping make DokTUI better. This file is a quick entry point; the full contributor handbook is in [`docs/PROJECT-GUIDE.md`](docs/PROJECT-GUIDE.md).

## What should I read first?

- [`docs/PROJECT-GUIDE.md`](docs/PROJECT-GUIDE.md) — architecture, tech stack, project overview, glossary.
- [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) — day-to-day dev commands, common tasks, and troubleshooting.
- [`docs/TESTING.md`](docs/TESTING.md) — testing locally without a remote VPS.
- [`AGENTS.md`](AGENTS.md) — verification commands used by Devin / automated agents.
- [`README.md`](README.md) — user-facing features, install, and keybindings.

## One-minute setup

You need a recent stable Rust toolchain. Then:

```sh
git clone https://github.com/doklabs/doktui
cd doktui

# If you have `just` installed:
just dev

# Or with plain Cargo:
cargo run
```

## Common dev commands

We use a [`justfile`](justfile) so you do not have to memorize long commands.

```sh
just dev      # build and run
just test     # run tests
just check    # cargo check --all-targets
just clippy   # cargo clippy --all-targets
just fmt      # cargo fmt
just verify   # clippy + check + test
just lint     # fmt + clippy + check + test
```

If you prefer plain Cargo, run:

```sh
cargo fmt --check
cargo clippy --all-targets
cargo check --all-targets
cargo test --all-targets
```

## Workflow

1. **Fork** the repository and create a branch from `main`:
   ```sh
   git checkout -b feat/your-thing
   ```
2. **Make your changes**. Follow the conventions below.
3. **Run verification** before opening a PR:
   ```sh
   just verify
   ```
4. **Commit** with a clear message describing the *why*.
5. **Open a PR** with a clear description. Reference any relevant docs.

For larger changes, open an issue first to discuss the design.

## Code conventions

- **No hardcoded colors.** Always use `theme.color(Role::…)` or `theme.style(Role::…)`. See `docs/PROJECT-GUIDE.md` §11.
- **Follow The Elm Architecture:** input → `Message` → `update` → render.
- **i18n first.** UI strings live in `locales/en.ftl`. Add new keys there; use `i18n.t(...)` in code.
- **Add tests** for service logic and pure helpers (routing, cron, backoff, secrets, etc.).
- **Keep views focused.** Rendering code should not call I/O directly; it reads `AppState`.

## Adding a new screen

The typical path is:

1. Add a `Screen` variant in `src/app/state.rs`.
2. Add the `Message` variants you need in `src/app/event.rs`.
3. Wire key input in `map_key` (`src/app/mod.rs`).
4. Handle the messages in the `update` function (`src/app/mod.rs`).
5. Create a view file in `src/ui/views/`.
6. Wire it into `src/ui/mod.rs`.
7. Add i18n keys in `locales/en.ftl`.
8. Update the shortcut line at the bottom of the view.
9. Update `README.md` keyboard table if the screen exposes new global keys.

## Where can I help?

Good first issues are listed in `docs/PROJECT-GUIDE.md` §10. Quick wins:

- New theme: copy `themes/*.toml` and adjust.
- New mascot frame: `src/ui/sprite.rs`.
- Translation: copy `locales/en.ftl`.
- Documentation: improve these files or add examples.

## Code of conduct

Be respectful, be constructive, and assume good intent. Security issues should be reported privately via GitHub security advisories, not public issues.

## License

By contributing, you agree that your work will be licensed under the [MIT License](LICENSE).
