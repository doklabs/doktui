<div align="center">

# DokTUI

**A lightweight terminal UI for managing remote Docker servers over SSH.**
An open-source, local-first alternative to [Dokploy](https://dokploy.com) — by [Doklabs](https://github.com/doklabs).

[![CI](https://github.com/doklabs/doktui/actions/workflows/ci.yml/badge.svg)](https://github.com/doklabs/doktui/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/doklabs/doktui?sort=semver)](https://github.com/doklabs/doktui/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)

```
▓▒░ getting started
        ┌───────────────┐
        │  >_  DokTUI   │   local TUI for remote deployments
        └───────────────┘
   [1] Register   [2] Check Docker   [3] Deploy
```

</div>

---

## What is DokTUI?

DokTUI runs **on your machine**, not on your server. It manages remote servers over SSH from a fast, keyboard- (and mouse-) driven terminal UI. Your servers only need **Docker + Traefik** — there is no permanent web dashboard eating server RAM/CPU.

- 🦀 **Single Rust binary** — no runtime, no dependencies, small footprint.
- 🔌 **Local-first** — the control plane lives on your laptop; servers stay lean.
- 🔐 **Secure by design** — a dedicated SSH key, host-key verification, encrypted secrets at rest.
- 🎨 **Themeable pixel UI** — a retro, gamified look with a modular theme system.
- 🖥️ **Cross-platform** — macOS, Linux, and Windows, on amd64 and arm64.

> Status: early development (v0.x). Expect rapid changes.

---

## Features

- **Server management** — register SSH servers, see live connection status, auto-reconnect on drop.
- **One-command provisioning** — checks for Docker + Traefik and installs them remotely if missing.
- **Deploy** — Docker Compose deploys with automatic Traefik routing (domain, port, HTTPS) — no hand-written labels. See [`docs/TRAEFIK-ROUTING.md`](./docs/TRAEFIK-ROUTING.md).
- **Container management** — start/stop/restart/remove, live log streaming.
- **Encrypted secrets** — per-app env/secrets, encrypted at rest.
- **Built-in editor** — edit compose/env/config with Vim or non-Vim mode and syntax highlighting.
- **Schedules** — cron-style scheduled tasks.
- **Manual, verified updates** — `doktui update` with SHA-256 + minisign verification.

---

## Install

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/doklabs/doktui/main/scripts/install.sh | sh
```

Installs a prebuilt binary to `~/.local/bin` (make sure it's on your `PATH`). Override with env vars:

```sh
DOKTUI_INSTALL_DIR=/usr/local/bin DOKTUI_VERSION=v0.1.0 \
  curl -fsSL https://raw.githubusercontent.com/doklabs/doktui/main/scripts/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/doklabs/doktui/main/scripts/install.ps1 | iex
```

### Manual download

Grab the binary for your platform from the [Releases](https://github.com/doklabs/doktui/releases) page, verify the `.sha256`, and place it on your `PATH`. Supported targets:

| OS | amd64 | arm64 |
|----|-------|-------|
| Linux | `x86_64-unknown-linux-musl` | `aarch64-unknown-linux-gnu` |
| macOS | `x86_64-apple-darwin` | `aarch64-apple-darwin` |
| Windows | `x86_64-pc-windows-msvc` | `aarch64-pc-windows-msvc` |

No Rust toolchain is required to install or run DokTUI.

---

## Quick Start

```sh
doktui
```

1. **Register a server** — enter host, user, port, and an ACME email (for Let's Encrypt).
2. **Provision** — DokTUI checks the server. If Docker/Traefik are missing, it installs them and creates the shared `doktui-network`.
3. **Deploy** — go to Deployments, paste a `docker-compose.yml`, fill in domain + port + HTTPS, and deploy. DokTUI injects the Traefik routing for you.

On first run, DokTUI generates a **dedicated SSH key** (shown on the Welcome screen). Add its public key to your server's `~/.ssh/authorized_keys`.

### Updating

```sh
doktui update
```

Downloads the matching release binary, verifies its checksum/signature, and swaps it in place. If you installed via a package manager, DokTUI points you to that manager instead.

---

## Keyboard & Mouse

The UI is fully keyboard-driven, and clickable with the mouse (buttons, nav, scroll). Most keys are context-sensitive — the same key does different things depending on the active screen.

### Global

| Key | Action |
|-----|--------|
| `Ctrl+C` | Quit |
| `Ctrl+F` | Toggle search |
| `Ctrl+U` | Toggle UI mode (compact/overlay) |
| `F6` / `Ctrl+Tab` | Toggle focus between sidebar and body |

### Sidebar (when focused)

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Move selection |
| `Enter` / `l` / `→` | Open selected section |
| `h` / `Esc` / `←` | Return focus to body |
| `1`–`5` | Jump to Home/Projects/Deployments/Monitoring/Schedules |
| `[` / `]` | Narrow / widen sidebar |

### Body / list screens

| Key | Action |
|-----|--------|
| `j` / `k` | Move selection |
| `Enter` | Open selected item |
| `b` / `Esc` | Back |
| `q` | Quit |
| `x` | Remove/delete selected item |
| `a` | Add (server, cron job) |
| `c` | Connect to server (Projects) or Containers (Deployments) |
| `p` | Provision server (Projects) |
| `s` / `S` | Stop / start container (Containers) |
| `r` | Restart container (Containers) |
| `l` | Logs (Containers / Deployments) |
| `e` | Open compose editor (Deploy) |

### Forms (Add server, Deploy, Secrets, Cron form)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` or `j` / `k` | Move between fields |
| `Enter` | Save / submit |
| `Esc` | Cancel |
| `Space` | Toggle HTTPS (Deploy) |
| `Ctrl+X` | Delete last secret (Secrets) |

---

## Configuration

Config and data live in per-OS locations:

| | Path |
|--|------|
| macOS/Linux config | `~/.config/doktui/config.toml` |
| Windows config | `%APPDATA%\doktui\config.toml` |
| Data (keys, known_hosts, secrets) | OS data dir, e.g. `~/.local/share/doktui/` |

Common settings: `theme`, `locale`, `sidebar_width`, `editor_mode` (`vim`/`normal`), `auto_reconnect`, `check_updates`, `mouse`, `acme_email`.

### Locales

UI strings are loaded from Fluent (`.ftl`) locale files. English is bundled; add translations under `~/.config/doktui/locales/`:

```sh
cp locales/en.ftl ~/.config/doktui/locales/id.ftl
# edit translations, then set in config.toml:
# locale = "id"
```

### Themes

Themes are plain TOML files — add your own without recompiling:

```sh
cp themes/gruvbox-material.toml ~/.config/doktui/themes/my-theme.toml
# edit colors, then set in ~/.config/doktui/config.toml:
# theme = "my-theme"
doktui
```

See [`docs/DESIGN.md`](./docs/DESIGN.md) for the theme schema (semantic roles, glyphs, motion, mascot) and inheritance via `extends`.

---

## Build from Source

Requires a recent stable Rust toolchain.

```sh
git clone https://github.com/doklabs/doktui
cd doktui
cargo build --release      # binary at target/release/doktui
cargo run                  # run locally
cargo test                 # run the test suite
```

We also provide a `justfile` with common recipes. If you have [just](https://github.com/casey/just) installed:

```sh
just dev      # build and run
just test     # run all tests
just verify   # clippy + check + test
just lint     # fmt + clippy + check + test
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for the full developer workflow.

For a fully static Linux build:

```sh
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

---

## Project Layout

```
doktui/
├── src/
│   ├── main.rs         # entrypoint, CLI, terminal bootstrap
│   ├── app/            # The Elm Architecture core: state, events, commands, loop
│   ├── ui/             # views, layout, editor, theme, sprites, animation, components
│   ├── services/       # ssh, docker, traefik, provision, routing, secrets, cron, updater
│   ├── security/       # dedicated key, keychain, host-key verification
│   └── config/         # per-OS paths, load/save
├── themes/             # bundled TOML themes
├── scripts/            # install.sh / install.ps1
├── docs/               # PRD, TDD, design & implementation docs
└── .github/workflows/  # CI (check/test/musl) + release (multi-arch artifacts)
```

DokTUI follows **The Elm Architecture**: a single `AppState`, a `Message` enum, and an `update` function, with all I/O in background `tokio` tasks that talk to the core over an `mpsc` channel. See [`docs/TDD.md`](./docs/TDD.md).

---

## Documentation

| Doc | What it covers |
|-----|----------------|
| [PRD.md](./docs/PRD.md) | Product requirements, goals, scope |
| [TDD.md](./docs/TDD.md) | Technical design: architecture, modules, stack |
| [DESIGN.md](./docs/DESIGN.md) | Visual language, animation, modular theme system |
| [TRAEFIK-ROUTING.md](./docs/TRAEFIK-ROUTING.md) | Routing model, label generation, shared network |
| [LAYOUT-REVISION.md](./docs/LAYOUT-REVISION.md) | Onboarding/Home layout & responsive/compact mode |
| [INTERACTION-AND-POLISH.md](./docs/INTERACTION-AND-POLISH.md) | Mouse support, animation timing, pixel design |
| [PROJECT-GUIDE.md](./docs/PROJECT-GUIDE.md) | Full project handbook: setup, architecture, contribution, glossary |
| [DEVELOPMENT.md](./docs/DEVELOPMENT.md) | Day-to-day dev commands, common tasks, troubleshooting |
| [TESTING.md](./docs/TESTING.md) | Unit, mocked, integration, and CI testing without a remote VPS |

---

## Contributing

Contributions are welcome. Start with [`CONTRIBUTING.md`](CONTRIBUTING.md) for the short version, then read [`docs/PROJECT-GUIDE.md`](docs/PROJECT-GUIDE.md) for the full handbook and [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for day-to-day workflow commands.

Good first issues: adding a theme (`themes/*.toml`), a new mascot variant (`src/ui/sprite.rs`), or a view for an existing service. Please open an issue to discuss larger changes first.

---

## Security

DokTUI holds SSH access to production servers, so security is a priority: a dedicated Ed25519 key, `0600` key permissions (with OS-keychain support), host-key verification, secrets encrypted at rest, verified updates, and no silent telemetry. Please report vulnerabilities privately via a GitHub security advisory rather than a public issue.

---

## License

Licensed under the [MIT License](./LICENSE). © Doklabs and contributors.
