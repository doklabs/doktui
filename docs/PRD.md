# PRD — DokTUI

**Status:** Draft
**Date:** July 6, 2026
**Project:** Doklabs (open source)
**Version:** 0.1

---

## 1. Summary

DokTUI is an **open-source product by Doklabs** — a replacement for [Dokploy](https://dokploy.com) in the form of a leaner, more efficient **TUI (Terminal User Interface)** application. Instead of running a heavy web panel on the server, DokTUI runs **locally** on the user's machine and manages remote servers over SSH.

Core philosophy: **keep it simple**. Users install from a public repo, run DokTUI locally, register an SSH server, and manage deployments straight from the terminal — with a more interactive, gamified visual touch.

---

## 2. Background & Problem

Dokploy is a great self-hosted PaaS, but:

- It requires server resources to run its own web dashboard (Postgres, Redis, UI, etc.), consuming RAM/CPU on production servers.
- Operational overhead: you must keep the panel alive, updated, and securely exposed.
- For users managing only a handful of servers, this feels excessive.

DokTUI moves the control "brain" to the user's local side. The server only needs to run Docker + Traefik. No dashboard runs permanently on the server, so server resources are fully dedicated to production apps.

---

## 3. Objective Goals

- **Rust tech stack** to build the TUI (for performance, a single binary, and a small footprint).
- **Dokploy-like functionality**: app deployment, container management, environment variables, domain/routing via Traefik, logs, and basic monitoring.
- **Drop irrelevant features** because DokTUI runs locally (e.g., web-based multi-user systems, auth panels, billing, server-based team management).
- **Built-in canvas code editor** with two modes: **Vim** and **non-Vim**, to edit config/compose/env directly from the TUI.
- **Cross-platform**: runs on **macOS, Linux, and Windows** so everyone can benefit.

## 4. Non-Objective Goals

- **SSH auto-reconnect**: remote SSH connections must reconnect automatically so sessions never feel choppy; this behavior is **on by default**.
- **Ease of use**: the experience must stay as simple as possible.
- **Gamified & interactive look**: the UI may feel more alive, with gamification and interactivity.

> Note: items in this section are desired product properties/qualities rather than "core functional features" — they are design principles and default behaviors the implementation must satisfy.

---

## 5. User Personas

- **Solo developer / indie hacker** managing 1–5 VPSes who wants fast deploys without a heavy panel.
- **Small DevOps / lean team** comfortable in the terminal who values keyboard-driven workflows.
- **Ex-Dokploy users** who want a lighter but familiar alternative.

---

## 6. Onboarding Flow

The main flow is designed to be minimal and linear:

1. **Install** — users install DokTUI **directly, without needing cargo/the Rust toolchain**. Primary method: a one-line script (`curl -fsSL … | sh` on macOS/Linux, `irm … | iex` / an `.exe` installer / `winget`/`scoop` on Windows) that downloads a **prebuilt binary** from the public release repo. The script auto-detects OS & architecture and fetches the right binary.
2. **Run locally** — DokTUI runs on the user's local machine (`doktui`).
3. **Register remote SSH** — the user is guided to register their server's SSH connection (host, user, port, key/credential).
4. **Server check** — DokTUI checks whether **Docker** and **Traefik** are installed.
   - **If not** → DokTUI performs a **remote install** (installs Docker + Traefik over SSH).
   - **If yes** → the user is taken straight to the **DokTUI dashboard**.
5. **Dashboard** — the user starts managing deployments.

```
Install (public repo)
        │
        ▼
Run DokTUI locally
        │
        ▼
Register remote SSH
        │
        ▼
Check Docker + Traefik on server ──── present? ────► DokTUI dashboard
        │
        no
        │
        ▼
Install Docker + Traefik remotely ─────────────────► DokTUI dashboard
```

---

## 7. Features & Scope

### 7.1 Core Features (Dokploy-like)

- **Server management** — list SSH servers, connection status, health checks.
- **App deployment** — from a Git repo, Docker image, or Docker Compose.
- **Container management** — start/stop/restart/remove, view status.
- **Environment variables & secrets** — managed per app.
- **Domain & routing** — Traefik integration for domains, subdomains, and automatic TLS (Let's Encrypt).
- **Logs** — real-time container log streaming in the TUI.
- **Basic monitoring** — CPU, memory, and container status per server.

### 7.2 Canvas Code Editor

- Integrated editor for config files, `docker-compose.yml`, `.env`, and Dockerfile directly from the TUI.
- **Vim mode** and **non-Vim mode** (user-selectable).
- **Syntax highlighting** for YAML, TOML, ENV, Dockerfile, and JSON — available from the first release.

### 7.3 Connection Behavior

- **SSH auto-reconnect** on by default; remote sessions feel seamless even on unstable networks.
- Clear connection-status indicators in the UI.

### 7.4 Experience & Appearance

- Gamification is **limited to UI characters/visual aspects only** — e.g., a mascot/character, iconography, colors, and light animation on interface elements. There is **no** points system, levels, rewards, or functional achievements affecting the workflow.
- Keyboard-driven navigation with consistent, memorable shortcuts.
- Consistent across all platforms (macOS, Linux, Windows).

### 7.5 Binary Updates (Manual)

- **Updates are manual**; there is no silent auto-update. This is intentional: DokTUI manages production servers, so a sudden binary change mid-deploy is risky and reduces predictability.
- **Notify-on-launch**: at startup, DokTUI checks for the latest version asynchronously in the background (non-blocking, doesn't hold up startup) and shows a small notice if a newer version exists — e.g., "v0.3 available — run `doktui update`".
- **`doktui update` command**: downloads the release binary matching the OS/architecture, verifies integrity, swaps the binary in place, and shows a short changelog. Runs only on explicit user command.
- **Install-method detection**: if DokTUI was installed via a package manager (Homebrew/winget/scoop/AUR), `doktui update` steps aside and points the user to the package manager (`brew upgrade`, etc.) to avoid version conflicts. In-place self-update is only active for direct script installs.
- **Opt-out**: version checking can be disabled entirely via config, for air-gapped environments or users who don't want any outbound connection at startup.

### 7.6 Out of Scope (Removed Features)

Because DokTUI runs locally, the following are **not** provided:

- Web panel / browser-based dashboard on the server.
- Multi-user systems & server-based team management.
- Web-based auth/login panels.
- Billing / subscriptions.
- Background services running permanently on the server (aside from the user's own Docker & Traefik).

---

## 8. Technical Architecture (High-Level)

- **Language:** Rust.
- **TUI framework:** `ratatui` + `crossterm`.
- **SSH:** a Rust SSH library (`russh`) with an auto-reconnect layer on top.
- **Async runtime:** `tokio` to handle remote connections & log streaming concurrently.
- **Editor:** a custom editor component supporting Vim & non-Vim modes.
- **Local configuration:** stored on the user's machine (e.g., `~/.config/doktui/` on macOS/Linux, `%APPDATA%\doktui\` on Windows), with no server state.
- **Cross-platform & multi-architecture:** prebuilt binaries for macOS, Linux, and Windows on **amd64 (x86_64)** and **arm64 (aarch64)**. Releases are built via CI cross-compilation, without requiring the user to have a Rust toolchain. `crossterm` was chosen for cross-OS terminal portability; config paths & terminal handling are adjusted per platform.
- **SSH key:** DokTUI **generates a dedicated DokTUI key** (a dedicated keypair) during onboarding, separate from the user's system key, for isolation and easy access revocation.
- **Control model:** all orchestration runs locally → commands to the server via SSH → the server only runs Docker + Traefik.

```
┌────────────────────────────┐        SSH (auto-reconnect)        ┌──────────────────────┐
│   DokTUI (local, Rust)     │ ─────────────────────────────────► │   Remote server      │
│  - TUI (ratatui)           │                                    │   - Docker           │
│  - Editor (vim/non-vim)    │ ◄───────── logs / status ───────── │   - Traefik          │
│  - Local config            │                                    │   - App containers   │
└────────────────────────────┘                                    └──────────────────────┘
```

---

## 9. Security

Because DokTUI runs on the **user's local device** and holds SSH access to production servers, local-side security is a top priority. This binary is a high-value target — compromising it means compromising every managed server.

### 9.1 Local Credential Storage

- **SSH private keys** are stored with strict permissions (`0600` on macOS/Linux; equivalent ACLs on Windows). DokTUI refuses to run if key-file permissions are too loose.
- Ideally keys are secured via the **OS keychain/secret store** when available (macOS Keychain, Windows Credential Manager, `libsecret`/Secret Service on Linux), with a fallback to an encrypted file.
- **Passphrase-protected key** options and **ssh-agent** integration for users who don't want keys stored unprotected.
- **App secrets/env** managed by DokTUI are not stored as plaintext in config; they are encrypted at rest on the local device.

### 9.2 Binary & Update Integrity

- Every update verifies the **SHA-256 checksum** and, ideally, the release **signature** before swapping the binary. Updates are rejected if verification fails.
- The initial installer script also provides a verifiable checksum, and all downloads happen over HTTPS.
- Release builds run through CI that is as reproducible as possible, with signed artifacts.

### 9.3 Other Security Principles

- **Secure transport**: all server communication is over encrypted SSH; host-key verification (known_hosts) with a clear warning when a fingerprint changes (MITM mitigation).
- **Least privilege**: the dedicated DokTUI key makes revoking access easy without disrupting the user's system key.
- **No silent telemetry**: no user data is sent. The only default outbound connection is the version check, which can be opted out of (see 7.5).
- **Log redaction**: secrets and credentials are never written to logs or the UI.
- **Destructive-action confirmation**: risky operations (removing a container, overwriting config) require explicit confirmation.

---

## 10. Success Metrics

- **Time-to-first-deploy**: time from install to a first successful deploy (target: minutes, not hours).
- **Server footprint**: zero extra dashboard processes on the server beyond Docker/Traefik.
- **Connection stability**: SSH sessions recover automatically without user intervention on brief network disruptions.
- **Onboarding satisfaction**: users complete the onboarding flow without external documentation.

---

## 11. Open Questions

- Names of additional distribution channels (e.g., whether a Homebrew tap, AUR package, etc. are needed) beyond the primary installer script.

**Decided:**

- **Binary updates** → manual (`doktui update`), notify-on-launch, integrity verification, install-method detection, opt-out capable.
- **Install** → direct, without cargo; the installer script downloads a prebuilt binary from the public release repo.
- **Multi-architecture** → amd64 (x86_64) and arm64 (aarch64) supported from the first release, across macOS/Linux/Windows.
- **Editor syntax highlighting** → available from the first release (YAML, TOML, ENV, Dockerfile, JSON).
- **SSH key** → generate a dedicated DokTUI keypair, separate from the user's system key.
- **Gamification** → limited to UI characters/visuals, with no functional reward mechanics.
- **Platforms** → macOS, Linux, and Windows supported.
- **Release signature** → minisign for update verification.

---

## 12. Release Plan (Tentative)

- **v0.1 (MVP):** onboarding, SSH registration, check + install Docker/Traefik, basic deploy, logs, auto-reconnect, secure key storage + host-key verification, manual update (`doktui update`).
- **v0.2:** canvas code editor (Vim & non-Vim) with syntax highlighting, encrypted env/secrets management, basic monitoring.
- **v0.3:** UI character/visual polish, UX & shortcut improvements, full OS keychain integration.
