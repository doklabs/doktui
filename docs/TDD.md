# TDD — DokTUI

**Status:** Draft
**Date:** July 6, 2026
**Project:** Doklabs (open source)
**Version:** 0.1
**Reference:** [PRD.md](./PRD.md)

---

## 1. Purpose

This document details DokTUI's technical design: how components are implemented to satisfy the requirements in the PRD. It focuses on module architecture, library choices, data flow, the concurrency model, the SSH layer, the editor, security, and the build/release pipeline. Visual UI details are out of scope except where they affect architecture (see [DESIGN.md](./DESIGN.md)).

---

## 2. Architecture Overview

DokTUI is a **single-binary** Rust TUI application that runs on the user's local device. All orchestration happens locally; the remote server only runs Docker + Traefik and receives commands over SSH.

The architecture follows a **layered** pattern with a clear separation between UI, application state/logic, and I/O (SSH, filesystem, network).

```
┌──────────────────────────────────────────────────────────────┐
│                        DokTUI (local)                         │
│                                                                │
│  ┌──────────────┐   events    ┌───────────────────────────┐   │
│  │   UI Layer   │ ──────────► │   Application Core (State) │   │
│  │  (ratatui)   │ ◄────────── │   - state store            │   │
│  │  - views     │   render    │   - reducer/handler        │   │
│  │  - editor    │             │   - command dispatcher     │   │
│  └──────────────┘             └───────────┬───────────────┘   │
│                                            │ async commands    │
│                               ┌────────────▼───────────────┐   │
│                               │      Services Layer         │   │
│                               │  - SSH manager (reconnect)  │   │
│                               │  - Docker controller        │   │
│                               │  - Traefik provisioner      │   │
│                               │  - Secret/Config store      │   │
│                               │  - Updater                  │   │
│                               └────────────┬───────────────┘   │
└────────────────────────────────────────────┼──────────────────┘
                                             │ SSH (encrypted)
                                             ▼
                              ┌────────────────────────────┐
                              │   Remote server             │
                              │   Docker + Traefik          │
                              └────────────────────────────┘
```

---

## 3. Tech Stack

| Need | Choice | Rationale |
|---|---|---|
| Language | Rust (edition 2021) | Performance, single binary, small footprint, memory-safe |
| TUI framework | `ratatui` + `crossterm` | Cross-OS terminal portability (macOS/Linux/Windows) |
| Async runtime | `tokio` | Concurrency for many SSH connections & log streaming |
| SSH | `russh` (pure-Rust) | No C dependency, full control for auto-reconnect |
| Editor buffer | `ropey` (rope data structure) | Efficient for large text edits |
| Syntax highlighting | Custom line highlighter (`src/ui/editor/highlight.rs`) | Lightweight per-line rules for YAML/TOML/ENV/Dockerfile/JSON; no extra native deps |
| Config serialization | `serde` + `toml` | TOML-based local config |
| At-rest encryption | `chacha20poly1305` | Encrypt secrets & keys on the local device |
| OS keychain | `keyring` crate | Access Keychain/Credential Manager/Secret Service |
| Update verification | `minisign-verify` + `sha2` | Release integrity (SHA-256 + signature) |
| CLI args | `clap` | Argument parsing (`doktui update`, etc.) |
| Logging | `tracing` + `tracing-subscriber` | Structured logging with secret redaction |
| Cron | `cron` + `chrono` | Scheduled task expressions |

---

## 4. Module Structure

Crate structure (single workspace, split into modules):

```
doktui/
├── src/
│   ├── main.rs               # entrypoint, CLI parsing, runtime bootstrap
│   ├── app/
│   │   ├── mod.rs            # application core, event loop
│   │   ├── state.rs          # AppState, centralized data model
│   │   ├── event.rs          # Event & Message definitions
│   │   └── command.rs        # command dispatcher (async)
│   ├── ui/
│   │   ├── mod.rs            # root render
│   │   ├── views/            # dashboard, server list, logs, etc.
│   │   ├── editor/           # canvas editor (vim & non-vim)
│   │   ├── layout/           # shell, sidebar, overlays
│   │   ├── components.rs     # reusable widgets (button, card, health_bar)
│   │   ├── sprite.rs         # mascot sprites (half-block)
│   │   ├── anim.rs           # tick-based animation primitives
│   │   └── theme/            # modular theme system (see DESIGN.md)
│   ├── services/
│   │   ├── ssh/              # SSH manager + auto-reconnect
│   │   ├── docker.rs         # Docker commands over SSH
│   │   ├── traefik.rs        # Traefik provisioning
│   │   ├── provision.rs      # check & remote-install Docker/Traefik
│   │   ├── routing.rs        # Traefik label generation / compose injection
│   │   ├── secrets.rs        # secret encryption/decryption
│   │   ├── cron.rs           # schedule expression handling
│   │   └── updater.rs        # manual binary update
│   ├── config/
│   │   ├── mod.rs            # load/save local config
│   │   └── paths.rs          # per-OS paths
│   └── security/
│       ├── keys.rs           # generate & store the dedicated DokTUI SSH key
│       ├── keychain.rs       # OS keychain integration
│       └── hostkey.rs        # known_hosts verification
├── themes/                   # bundled TOML themes
└── Cargo.toml
```

---

## 5. Concurrency Model & Event Loop

DokTUI uses a **message-passing** architecture in the style of Elm/TEA (The Elm Architecture) on top of `tokio`.

- The **main loop** (single-threaded on the UI) handles terminal input and rendering.
- **Background tasks** (`tokio` tasks) handle I/O: SSH connections, log streaming, version checks, provisioning. They communicate with the core via `tokio::sync::mpsc` channels.
- Every event (keypress, SSH result, tick timer) becomes a `Message` placed on the queue; the core processes it, updates `AppState`, and triggers a re-render.

```
[terminal input] ─┐
[ssh events]      ├──► mpsc channel ──► Core.update(Message) ──► AppState ──► UI.render()
[timer ticks]     ┘
```

Benefit: rendering is never blocked by network I/O, so the UI stays responsive (supporting the "never feels choppy" goal). See [INTERACTION-AND-POLISH.md](./INTERACTION-AND-POLISH.md) for the frame/housekeeping tick split.

---

## 6. SSH Layer & Auto-Reconnect

This is the most UX-critical component. Design:

- **Connection pool**: one persistent connection per registered server, managed by `SshManager`.
- **Connection state machine**: `Disconnected → Connecting → Connected → Reconnecting`. Transitions are published to the UI as status indicators.
- **Auto-reconnect (on by default)**: when a connection drops, the manager automatically retries with **exponential backoff + jitter** (e.g., 1s, 2s, 4s, … up to a maximum), without user intervention.
- **Keep-alive**: periodic SSH keep-alive/heartbeat to detect dead connections faster.
- **Command queue**: commands sent while the connection is down are queued (or marked failed with retry) so the session feels seamless.
- **Multiplexing**: use separate SSH channels for log streaming vs. command execution, so real-time logs don't block interactive commands.

Host-key verification is done in `security/hostkey.rs` against the local `known_hosts`; a fingerprint change triggers a warning (MITM mitigation).

---

## 7. Remote Provisioning (Check & Install Docker/Traefik)

Flow in `services/provision.rs`:

1. After SSH connects, run probes: `command -v docker`, `docker compose version`, and check the Traefik container/service.
2. If **absent**:
   - Detect the server OS/distro (`/etc/os-release`).
   - Run the Docker install (e.g., the official `get.docker.com` script) over SSH.
   - Deploy Traefik as a container with a default configuration (entrypoints, Docker provider, Let's Encrypt TLS resolver).
   - Verify the install result.
3. If **present** → continue to the dashboard.

Each step shows progress in the UI; failures show an actionable message, not a raw stack trace.

> Note: Traefik provisioning also creates a shared `doktui-network` and can auto-migrate a legacy Traefik install — see [TRAEFIK-ROUTING.md](./TRAEFIK-ROUTING.md).

---

## 8. Deploy & Container Management

- **Deploy sources**: Git repo (clone/pull on the server), Docker image (pull), or Docker Compose (upload/sync a compose file).
- **Execution**: DokTUI composes `docker`/`docker compose` commands and sends them over SSH; output is streamed back.
- **File transport**: compose and `.env` are streamed via `write_remote_file` (`cat > path` with chunking) to avoid shell `ARG_MAX` limits.
- **Traefik routing**: Traefik labels are injected into the compose (host rule, entrypoint, TLS) based on the user-configured domain, and the service is attached to `doktui-network`. See [TRAEFIK-ROUTING.md](./TRAEFIK-ROUTING.md).
- **Env & secrets**: injected at runtime; sensitive values are never written as plaintext to server config or logs.
- **Post-deploy verification**: after `docker compose up`, DokTUI verifies the container is running and reports routing status.
- **Logs**: streamed via a dedicated SSH channel.

---

## 9. Canvas Code Editor

- **Buffer**: a `ropey` structure for efficient edits.
- **Modes**: `Vim` (modal: normal/insert/visual, a subset of common motions & commands) and `non-Vim` (standard editing). The mode is chosen in config and can be switched at runtime.
- **Syntax highlighting**: a custom per-line highlighter in `src/ui/editor/highlight.rs` for YAML, TOML, ENV, Dockerfile, and JSON. Highlighting is computed per visible line so it does not block rendering.
- **Target files**: config, `docker-compose.yml`, `.env`, Dockerfile — both local files and remote files fetched over SSH and synced back.
- **Security**: when editing files containing secrets, the editor honors the log-redaction rules and does not write the buffer to unprotected temporary locations.

---

## 10. Configuration & Local Storage

- **Config location**: `~/.config/doktui/` (macOS/Linux), `%APPDATA%\doktui\` (Windows) — resolved by `config/paths.rs` (the `directories` crate).
- **Format**: TOML via `serde`.
- **Contents**: server list, editor preferences (vim/non-vim), UI theme, update opt-out flag, ACME email.
- **Secrets & keys**: NOT stored as plaintext. The dedicated DokTUI private key and app secrets are encrypted at rest (`chacha20poly1305`), with the option to store in the OS keychain via the `keyring` crate.
- **Permissions**: key files are forced to `0600` (macOS/Linux) / equivalent ACLs (Windows); DokTUI refuses to run if permissions are too loose.

---

## 11. Security (Implementation)

Referencing PRD §9, the technical implementation:

- **Generate a dedicated DokTUI key** during onboarding (`security/keys.rs`) — an Ed25519 keypair, separate from the user's system key, easy to revoke.
- **Storage**: OS keychain first; encrypted-file fallback. Support for passphrase-protected keys & `ssh-agent` integration.
- **Update integrity**: verify SHA-256 + minisign signature before swapping the binary; reject on failure. All downloads over HTTPS.
- **Host-key verification**: local `known_hosts`, warning on fingerprint change.
- **Log redaction**: the `tracing` layer filters secrets before writing.
- **No telemetry**: the only default outbound connection is the version check (opt-out available).
- **Destructive-action confirmation**: removing a container / overwriting config requires explicit confirmation.

---

## 12. Updater (Manual)

`services/updater.rs`:

- **Notify-on-launch**: an async task checks the release endpoint (e.g., the GitHub Releases API) at startup, non-blocking; shows a notice if a newer version exists.
- **`doktui update`**: download the binary matching the OS/architecture → verify checksum & signature → swap in place (atomic rename) → show changelog.
- **Install-method detection**: read an install marker; if installed via a package manager (Homebrew/winget/scoop/AUR), point to the package manager instead of self-updating.
- **Opt-out**: a config flag disables all version checking.

---

## 13. Build, Release & Distribution

- **Cross-compilation** via CI for the matrix:
  - macOS: `x86_64-apple-darwin`, `aarch64-apple-darwin`
  - Linux: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-gnu` (musl for maximum static portability)
  - Windows: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`
- **Artifacts**: prebuilt binary + checksum file + signature per target, published to the public release repo.
- **Installer**:
  - macOS/Linux: a `curl -fsSL … | sh` script that detects OS/arch, downloads the binary, verifies the checksum, and places it on PATH.
  - Windows: an `irm … | iex` script / `.exe` installer, plus `winget`/`scoop` options.
- **No Rust toolchain** required on the user's side.

CI lives in `.github/workflows/` (`ci.yml` for multi-OS check/test + musl build, `release.yml` for tagged multi-arch artifacts).

---

## 14. Error Handling & Resilience

- Network/SSH errors are handled by the reconnect state machine (§6), not by crashing.
- Remote command failures are shown with context (command, exit code, curated stderr).
- A panic in a background task must not bring down the UI; tasks are isolated and reported as an error state.
- Corrupt config → fall back to defaults with a warning, rather than failing entirely.

---

## 15. Testing Strategy

- **Unit tests**: probe parsers, Docker/Traefik command assembly, reconnect backoff logic, secret encryption/decryption, Traefik label generation, cron expressions.
- **Integration tests**: the SSH manager against a test Docker server (an SSHD container) — including a dropped-connection scenario to validate auto-reconnect.
- **Editor tests**: buffer operations (`ropey`), Vim motions, highlighting.
- **Security tests**: update checksum/signature verification is rejected when tampered; key-permission enforcement; log redaction.
- **Cross-platform CI**: run tests on macOS, Linux, Windows.
- **Release verification**: smoke-test the cross-compiled binary per target.

---

## 16. Technical Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Cross-OS terminal behavior differences (especially Windows) | Broken/inconsistent UI | Rely on `crossterm`; per-platform CI tests; avoid non-portable terminal features |
| Auto-reconnect hides real network problems | User confusion | Clear status indicators + inspectable connection logs |
| Vim mode complexity | Scope creep | Start with a common motion/command subset, expand gradually |
| Local key-storage security | Server compromise | OS keychain + at-rest encryption + permission enforcement |
| Remote Docker install fails on an unknown distro | Onboarding stuck | OS detection + fallback message + support for the "Docker already present" scenario |

---

## 17. Resolved Technical Decisions

- **SSH library**: `russh` (pure-Rust) — no C dependency, simplifies static musl builds.
- **Syntax highlighting**: custom line-based highlighter (see `highlight.rs`); `tree-sitter` remains a future option if richer parsing is needed.
- **Release signature**: `minisign` (verified with `minisign-verify`).
- **Linux static build**: musl is the default target for maximum portability.
