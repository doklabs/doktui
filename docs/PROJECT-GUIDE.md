# DokTUI — Project Guide & Contribution Handbook

**Date:** July 12, 2026
**Project:** Doklabs — DokTUI (open source)
**Purpose:** A thorough explanation of what DokTUI is, how it works, and how you can contribute to it.

---

## Table of Contents

1. [What is DokTUI?](#1-what-is-doktui)
2. [Why Does DokTUI Exist?](#2-why-does-doktui-exist)
3. [How Does DokTUI Work?](#3-how-does-doktui-work)
4. [Project Architecture](#4-project-architecture)
5. [Key Features Explained](#5-key-features-explained)
6. [Tech Stack Breakdown](#6-tech-stack-breakdown)
7. [Project Folder Structure](#7-project-folder-structure)
8. [How to Set Up for Development](#8-how-to-set-up-for-development)
9. [How to Contribute](#9-how-to-contribute)
10. [Contribution Areas & Ideas](#10-contribution-areas--ideas)
11. [Code Conventions & Rules](#11-code-conventions--rules)
12. [Glossary of Technical Terms](#12-glossary-of-technical-terms)

---

## 1. What is DokTUI?

DokTUI is an **open-source terminal application** built by Doklabs. It lets you **manage remote servers, deploy apps, and handle Docker containers** — all from a beautiful, retro-styled terminal interface that runs on your own computer.

Think of it as a **lightweight replacement for Dokploy** (a web-based server management panel). Instead of running a heavy web dashboard *on your server* (which eats up server resources), DokTUI runs *locally on your laptop/desktop* and talks to your servers over SSH.

> **In simple terms:** DokTUI is a fancy terminal app that lets you deploy websites and apps to your servers without needing a web browser or a dashboard running on the server.

---

## 2. Why Does DokTUI Exist?

### The Problem

Tools like Dokploy are great, but they have downsides:

- **They run on your server** — this means your production server is spending RAM and CPU running a dashboard (Postgres, Redis, a web UI) instead of your actual apps.
- **Operational overhead** — you have to keep the dashboard alive, secure, and updated.
- **Overkill for small setups** — if you only manage 1–5 servers, a full web panel is excessive.

### The Solution

DokTUI moves the "brain" to your local machine. Your server only needs to run **Docker** (to run your apps) and **Traefik** (to route web traffic). No dashboard runs permanently on the server = more server resources for your actual apps.

---

## 3. How Does DokTUI Work?

The flow is simple:

```
You install DokTUI on your computer
        │
        ▼
You run `doktui` in your terminal
        │
        ▼
You register your remote server (SSH connection)
        │
        ▼
DokTUI checks if Docker + Traefik are installed
        │                                    │
    Not installed                        Installed
        │                                    │
        ▼                                    ▼
DokTUI installs them remotely ──────► Dashboard appears
                                     You manage everything!
```

Everything happens over **SSH** (a secure, encrypted connection). DokTUI sends commands to your server (like "start this container" or "show me the logs"), and the server sends results back.

---

## 4. Project Architecture

DokTUI uses a **three-layer architecture**:

### Layer 1: UI Layer (what you see)
- Built with `ratatui` (a Rust TUI framework)
- Renders views: dashboard, server list, logs, code editor
- Handles keyboard/mouse input
- Uses a **theme system** — all colors come from theme files, never hardcoded

### Layer 2: Application Core (the brain)
- Manages the **application state** (what servers exist, what's selected, etc.)
- Processes **messages** (events like "user pressed Enter" or "SSH connected")
- Follows **The Elm Architecture (TEA)** — a pattern where: input → message → update state → re-render

### Layer 3: Services Layer (the workers)
- **SSH Manager** — handles connections, auto-reconnect
- **Docker Controller** — sends Docker commands over SSH
- **Traefik Provisioner** — sets up web routing
- **Secrets Manager** — encrypts/decrypts sensitive data
- **Updater** — handles binary updates

```
┌──────────────────────────────────────────────┐
│              DokTUI (your computer)          │
│                                              │
│  UI Layer ──► Application Core ──► Services  │
└──────────────────────┬───────────────────────┘
                       │ SSH (encrypted)
                       ▼
              ┌────────────────┐
              │ Remote Server  │
              │ Docker+Traefik │
              └────────────────┘
```

---

## 5. Key Features Explained

### 5.1 Server Management
Register your SSH servers, see their connection status in real-time, and enjoy **auto-reconnect** — if your network hiccups, DokTUI reconnects automatically.

### 5.2 App Deployment
Deploy from a **GitHub repo** (clone/pull over SSH + `GITHUB_TOKEN`) or a pasted Docker Compose file. Apps are persisted locally; GitHub apps can **auto-deploy** by polling commit SHAs while DokTUI is open (not a 24/7 server webhook). DokTUI injects **Traefik routing labels** so your app is reachable via your domain with HTTPS.

### 5.3 Canvas Code Editor
A built-in text editor right in the terminal! Supports **Vim mode** and **non-Vim mode**, with syntax highlighting for YAML, TOML, ENV, Dockerfile, and JSON.

### 5.4 Modular Theme System
Themes are TOML files. You can create your own theme by copying an existing one and changing colors — **no recompiling needed**. Themes support **inheritance** (override only what you want to change).

### 5.5 Security
- Generates a **dedicated SSH key** (separate from your system key)
- Secrets are **encrypted at rest** using ChaCha20-Poly1305
- Host-key verification prevents man-in-the-middle attacks
- Updates are verified with SHA-256 checksums + minisign signatures
- **No telemetry** — the only outbound connection is an optional version check

### 5.6 Mascot & Gamification
A pixel-art mascot called "Doko" (a terminal crate) lives in the UI. Gamification is **visual only** — animated sprites, colored status indicators, and a retro aesthetic. No points, levels, or rewards.

---

## 6. Tech Stack Breakdown

| Technology | What It Does | Why It Was Chosen |
|---|---|---|
| **Rust** | The programming language | Fast, memory-safe, compiles to a single binary |
| **ratatui** + **crossterm** | TUI framework | Cross-platform terminal rendering (macOS/Linux/Windows) |
| **tokio** | Async runtime | Handles multiple SSH connections and log streams concurrently |
| **russh** | SSH library | Pure-Rust (no C dependencies), full control for auto-reconnect |
| **ropey** | Text buffer | Efficient rope data structure for the code editor |
| **serde** + **toml** | Config serialization | Read/write TOML config files |
| **chacha20poly1305** | Encryption | Encrypt secrets and keys stored locally |
| **keyring** | OS keychain access | Store keys in macOS Keychain / Windows Credential Manager |
| **clap** | CLI argument parsing | Handle `doktui update`, `doktui --theme`, etc. |
| **tracing** | Structured logging | Logs with automatic secret redaction |
| **fluent** | Internationalization (i18n) | UI strings in translatable `.ftl` files |

---

## 7. Project Folder Structure

```
doktui/
├── src/                        # All Rust source code
│   ├── main.rs                 # Entry point, CLI parsing, runtime bootstrap
│   ├── app/                    # Application core (The Elm Architecture)
│   │   ├── mod.rs              # Event loop
│   │   ├── state.rs            # AppState — the centralized data model
│   │   ├── event.rs            # Message/Event definitions
│   │   └── command.rs          # Async command dispatcher
│   ├── ui/                     # Everything visual
│   │   ├── views/              # Screen views (dashboard, onboarding, logs, etc.)
│   │   ├── editor/             # Canvas code editor (Vim + non-Vim modes)
│   │   ├── layout/             # Shell layout, sidebar, overlays
│   │   ├── theme/              # Modular theme system
│   │   ├── components.rs       # Reusable widgets (button, card, health_bar)
│   │   ├── sprite.rs           # Mascot pixel sprites
│   │   └── anim.rs             # Tick-based animation
│   ├── services/               # Backend logic
│   │   ├── ssh/                # SSH connection manager + auto-reconnect
│   │   ├── docker.rs           # Docker commands sent over SSH
│   │   ├── traefik.rs          # Traefik installation/provisioning
│   │   ├── routing.rs          # Traefik label generation + compose injection
│   │   ├── provision.rs        # Remote Docker/Traefik installation
│   │   ├── secrets.rs          # Secret encryption/decryption
│   │   ├── cron.rs             # Scheduled tasks
│   │   └── updater.rs          # Manual binary updates
│   ├── config/                 # Configuration management
│   │   ├── mod.rs              # Load/save config
│   │   └── paths.rs            # OS-specific paths
│   ├── security/               # Security layer
│   │   ├── keys.rs             # Dedicated SSH key generation
│   │   ├── keychain.rs         # OS keychain integration
│   │   └── hostkey.rs          # known_hosts verification
│   └── i18n/                   # Internationalization helpers
├── themes/                     # Bundled TOML theme files
│   ├── gruvbox-material.toml   # Default theme (warm, earthy)
│   └── pico8.toml              # Retro colorful theme
├── locales/                    # Translation files
│   └── en.ftl                  # English (bundled default)
├── scripts/                    # Installation scripts
│   ├── install.sh              # macOS/Linux installer
│   └── install.ps1             # Windows installer
├── docs/                       # Project documentation
│   ├── PRD.md                  # Product Requirements Document
│   ├── TDD.md                  # Technical Design Document
│   ├── DESIGN.md               # Visual/theme/animation system design
│   ├── TRAEFIK-ROUTING.md      # Traefik routing implementation
│   ├── LAYOUT-REVISION.md      # UI layout decisions
│   └── INTERACTION-AND-POLISH.md # Mouse, animation, pixel design
├── .github/workflows/          # CI/CD pipelines
├── Cargo.toml                  # Rust project manifest (dependencies)
├── Cargo.lock                  # Locked dependency versions
├── LICENSE                     # MIT License
└── README.md                   # Project overview
```

---

## 8. How to Set Up for Development

### Prerequisites
- A recent **stable Rust toolchain** (install from [rustup.rs](https://rustup.rs))
- Git

### Steps

```sh
# 1. Clone the repository
git clone https://github.com/doklabs/doktui
cd doktui

# 2. Build the project
cargo build

# 3. Run the application
cargo run

# 4. Run tests
cargo test

# 5. Check code formatting
cargo fmt --check

# 6. Run the linter
cargo clippy
```

### For a Release Build

```sh
cargo build --release
# Binary will be at: target/release/doktui
```

### For a Static Linux Build (musl)

```sh
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

---

## 9. How to Contribute

### Step-by-Step Workflow

1. **Fork the repository** on GitHub.
2. **Create a branch** from `main`:
   ```sh
   git checkout -b feat/your-feature-name
   ```
3. **Make your changes** — follow the code conventions in §11.
4. **Test your changes**:
   ```sh
   cargo build && cargo test
   ```
5. **Format and lint**:
   ```sh
   cargo fmt
   cargo clippy
   ```
6. **Commit** with a clear message:
   ```sh
   git commit -m "feat: add gruvbox-dark theme variant"
   ```
7. **Push and open a Pull Request** on GitHub with a clear description. Reference relevant docs if applicable.

### For Larger Changes

Open a **GitHub Issue** first to discuss the design before writing code. This prevents wasted effort if the approach needs adjustment.

---

## 10. Contribution Areas & Ideas

### 🟢 Beginner-Friendly (Good First Issues)

| Area | What to Do | Files to Touch |
|------|-----------|----------------|
| **New theme** | Copy an existing theme TOML, change colors/glyphs | `themes/*.toml` |
| **New mascot variant** | Add a new sprite frame (idle, happy, etc.) | `src/ui/sprite.rs` |
| **Translation** | Copy `locales/en.ftl`, translate to your language | `locales/*.ftl` |
| **Documentation** | Improve docs, fix typos, add examples | `docs/*.md`, `README.md` |

### 🟡 Intermediate

| Area | What to Do | Files to Touch |
|------|-----------|----------------|
| **New UI view** | Add a screen for an existing service | `src/ui/views/` |
| **Editor improvements** | Add Vim motions or highlighting rules | `src/ui/editor/` |
| **Cron UI** | Build the schedule management view | `src/ui/views/`, `src/services/cron.rs` |
| **Tests** | Add unit tests for routing labels, backoff logic, secrets | `src/services/` |

### 🔴 Advanced

| Area | What to Do | Files to Touch |
|------|-----------|----------------|
| **SSH auto-reconnect** | Improve the reconnect state machine | `src/services/ssh/` |
| **DNS-01 challenge** | Add Cloudflare DNS verification for wildcard certs | `src/services/traefik.rs` |
| **OS keychain** | Integrate with macOS Keychain / Windows Credential Manager | `src/security/keychain.rs` |
| **Cross-platform fixes** | Fix terminal rendering issues on Windows | Various |

---

## 11. Code Conventions & Rules

### The Golden Rule: No Hardcoded Colors

```rust
// ❌ WRONG — never do this in a view:
let style = Style::default().fg(Color::Rgb(90, 197, 79));

// ✅ CORRECT — always use theme roles:
let style = Style::default().fg(theme.color(Role::Success));
```

Views reference **semantic roles** (`Role::Success`, `Role::Danger`, `Role::Primary`), not raw RGB values. This makes themes work correctly.

### Architecture Pattern

Follow **The Elm Architecture (TEA)**:
1. User input or I/O result becomes a `Message`
2. The `update` function processes the `Message` and mutates `AppState`
3. The UI re-renders based on the new `AppState`

### Adding a New Screen

1. Add a variant to the `Screen` enum
2. Add corresponding `Message` variants in `src/app/event.rs`
3. Create a view file in `src/ui/views/`
4. Wire it into the render function in `src/ui/mod.rs`

### Code Quality

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Write tests for new service logic
- Keep functions focused and small

---

## 12. Glossary of Technical Terms

> Every technical word used in this project, explained in plain language.

---

### A

**ACME (Automatic Certificate Management Environment)**
> A protocol used by Let's Encrypt to automatically issue and renew SSL/TLS certificates. DokTUI uses ACME so your deployed apps get HTTPS automatically without manual certificate management.

**API (Application Programming Interface)**
> A set of rules that lets different software programs talk to each other. For example, DokTUI uses the GitHub Releases API to check for new versions.

**Architecture (Software Architecture)**
> The high-level structure of a software system — how its parts are organized and how they communicate. DokTUI uses a "layered architecture" with UI, Core, and Services layers.

**ARM64 / aarch64**
> A CPU architecture used by Apple Silicon Macs (M1/M2/M3/M4), Raspberry Pi, and many modern servers. DokTUI provides prebuilt binaries for this architecture.

**Artifacts (Build Artifacts)**
> The files produced by a build process — in DokTUI's case, the compiled binary files for each operating system and architecture.

**Async / Asynchronous**
> A programming pattern where tasks can run without blocking each other. For example, DokTUI can stream logs from a server while still responding to your keyboard input, because these tasks run asynchronously.

**Auto-reconnect**
> A feature where DokTUI automatically re-establishes an SSH connection if it drops (e.g., due to a network hiccup), without you having to do anything.

---

### B

**Backoff (Exponential Backoff)**
> A retry strategy where each successive retry waits longer: 1 second, then 2, then 4, then 8, etc. DokTUI uses this when reconnecting SSH to avoid overwhelming the server.

**Binary (Single Binary)**
> A compiled, standalone executable file. DokTUI compiles to a single binary — one file that you download and run, with no other files or runtimes needed.

---

### C

**ChaCha20-Poly1305**
> A modern encryption algorithm used by DokTUI to encrypt secrets (like passwords and API keys) stored on your local machine. It's fast and secure.

**CI (Continuous Integration)**
> An automated system that builds and tests code every time changes are pushed. DokTUI uses GitHub Actions for CI to ensure the code works on macOS, Linux, and Windows.

**CLI (Command Line Interface)**
> A text-based way to interact with a program by typing commands. `doktui update` is a CLI command.

**Clippy**
> Rust's official linting tool. It catches common mistakes, suggests improvements, and enforces best practices. Contributors should run `cargo clippy` before submitting code.

**Compose / Docker Compose**
> A tool for defining multi-container Docker applications using a YAML file (`docker-compose.yml`). You describe your app's services, and Docker Compose starts them all together.

**Concurrency**
> The ability to handle multiple tasks at the same time. DokTUI uses concurrency to manage several SSH connections and stream logs simultaneously.

**Crate (Rust Crate)**
> Rust's term for a package or library. The `Cargo.toml` file lists all the crates (dependencies) that DokTUI uses, like `ratatui`, `tokio`, and `russh`.

**Cross-compilation**
> Building software on one platform (e.g., Linux) that will run on another (e.g., macOS or Windows). DokTUI's CI cross-compiles binaries for 6 different targets.

**Crossterm**
> A Rust library for cross-platform terminal manipulation — handling keyboard input, mouse events, colors, and screen control on macOS, Linux, and Windows.

---

### D

**DNS-01 Challenge**
> A method of proving domain ownership to Let's Encrypt by creating a DNS record. This is needed for wildcard certificates. (Future feature for DokTUI.)

**Docker**
> A platform for running applications in isolated "containers." Each container is like a lightweight, portable virtual machine that holds an app and all its dependencies.

**Docker Provider (Traefik)**
> A Traefik configuration that tells Traefik to discover services by reading Docker container labels, instead of requiring manual configuration files.

---

### E

**Ed25519**
> A modern, fast, and secure algorithm for generating SSH key pairs. DokTUI generates an Ed25519 keypair during onboarding.

**Elm Architecture / TEA (The Elm Architecture)**
> A design pattern (from the Elm programming language) where: (1) all app data lives in a single State, (2) events become Messages, (3) an update function processes messages and modifies state, (4) the UI renders from state. DokTUI follows this pattern.

**Entrypoint (Traefik Entrypoint)**
> A network address where Traefik listens for incoming traffic. DokTUI configures two: `web` (port 80, HTTP) and `websecure` (port 443, HTTPS).

**ENV / .env File**
> A file containing environment variables (key=value pairs) used to configure an application. For example, `DATABASE_URL=postgres://...`.

**External Network (Docker)**
> A Docker network created outside of a compose file and shared between multiple compose projects. DokTUI creates `doktui-network` so Traefik can communicate with all deployed apps.

---

### F

**Fluent**
> Mozilla's localization system for translating UI text. DokTUI uses Fluent `.ftl` files so the interface can be translated to different languages.

**Fork (GitHub Fork)**
> A personal copy of someone else's GitHub repository. You fork a project, make changes in your copy, then submit a Pull Request to merge your changes back.

**FPS (Frames Per Second)**
> How many times the screen redraws per second. DokTUI targets ~15 FPS for smooth animations.

---

### G

**Gamification**
> Adding game-like elements to non-game software. In DokTUI, gamification is purely visual — animated mascots, retro pixel aesthetics, and colorful indicators. There are no points, levels, or rewards.

**Glyph**
> A visual symbol or character. DokTUI uses Unicode block characters like `█`, `░`, `▀`, `●` as "glyphs" to build its pixel-art interface.

**Gruvbox**
> A popular color scheme known for warm, earthy tones. DokTUI's default theme is "gruvbox-material."

---

### H

**Half-block Rendering**
> A technique using the `▀` character (upper half block) where the foreground color represents the top pixel and the background color represents the bottom pixel. This doubles the vertical resolution of terminal graphics.

**Host-key Verification**
> When connecting via SSH, the server presents a "fingerprint." DokTUI checks this against a `known_hosts` file to ensure you're connecting to the real server and not an impersonator (MITM attack).

**HTTP-01 Challenge**
> A method of proving domain ownership to Let's Encrypt by serving a specific file on port 80. This is what DokTUI uses by default for automatic HTTPS certificates.

---

### I

**i18n (Internationalization)**
> Making software adaptable to different languages and regions. The name "i18n" comes from "i-nternationalizatio-n" (18 letters between i and n).

**Idempotent**
> An operation that produces the same result whether you run it once or multiple times. For example, `ensure_network` creates the Docker network only if it doesn't already exist.

---

### J

**Jitter (in Backoff)**
> Adding a small random delay to retry timing so that multiple clients don't all retry at the exact same moment and overwhelm the server.

---

### K

**Keychain / Keyring**
> The operating system's built-in secure storage for passwords and keys. macOS has "Keychain," Windows has "Credential Manager," and Linux has "Secret Service." DokTUI can store SSH keys here.

---

### L

**Labels (Docker/Traefik Labels)**
> Key-value metadata attached to Docker containers. Traefik reads these labels to know how to route web traffic to each container (which domain, which port, whether to use HTTPS).

**Let's Encrypt**
> A free, automated certificate authority that issues SSL/TLS certificates. DokTUI uses Let's Encrypt (via the ACME protocol) to give your deployed apps HTTPS automatically.

**Lint / Linter**
> A tool that analyzes code for potential errors, style issues, and best practices. Rust's linter is called Clippy.

---

### M

**Mascot ("Doko")**
> DokTUI's pixel-art character — a shipping crate with a terminal screen face showing a `>` prompt. It has animation frames: idle, blinking, typing, success, and CRT glitch.

**Minisign**
> A simple tool for signing and verifying files. DokTUI uses minisign signatures to verify that downloaded updates haven't been tampered with.

**MITM (Man-in-the-Middle)**
> An attack where someone secretly intercepts communication between two parties. Host-key verification in SSH protects against this.

**Module (Rust Module)**
> A way to organize Rust code into separate files and namespaces. Each folder in `src/` (like `app/`, `ui/`, `services/`) is a module.

**mpsc (Multi-Producer, Single-Consumer)**
> A type of channel where many senders can send messages to one receiver. DokTUI uses `tokio::sync::mpsc` channels so background tasks (SSH, timers) can send messages to the main application core.

**Multiplexing (SSH)**
> Running multiple independent data streams over a single SSH connection. DokTUI uses separate SSH channels for log streaming and command execution so they don't block each other.

**musl**
> An alternative C standard library for Linux. Building with musl produces a **fully static binary** that runs on any Linux distribution without needing shared libraries.

---

### P

**PaaS (Platform as a Service)**
> A service that handles the infrastructure for deploying apps (like Heroku or Dokploy). DokTUI provides PaaS-like functionality but runs locally.

**PR (Pull Request)**
> A request to merge your code changes into the main project. You create a PR on GitHub after pushing your branch.

**PRD (Product Requirements Document)**
> A document that defines what a product should do, who it's for, and what features it needs. DokTUI's PRD is at `docs/PRD.md`.

**Provisioning**
> The process of setting up a server with the software it needs. DokTUI "provisions" a server by installing Docker and Traefik remotely over SSH.

---

### R

**Ratatui**
> A Rust library for building rich terminal user interfaces. It handles rendering text, colors, layouts, and widgets in the terminal.

**Rope (Data Structure)**
> A tree-based data structure for efficiently handling large strings with frequent edits (insertions, deletions). DokTUI's editor uses the `ropey` crate for this.

**Router (Traefik Router)**
> A Traefik concept that matches incoming web requests (by domain, path, etc.) and forwards them to the correct backend service.

**Rust**
> A systems programming language focused on performance, reliability, and memory safety. DokTUI is written entirely in Rust.

---

### S

**Semantic Role (Theme)**
> A named purpose for a color, like `Success` (green), `Danger` (red), or `Primary` (brand color). Views use semantic roles instead of raw colors, so themes can change all colors by remapping roles.

**Serde**
> A Rust framework for **ser**ializing (converting data to text/bytes) and **de**serializing (converting text/bytes back to data). Used for reading/writing TOML config files.

**SHA-256**
> A cryptographic hash function that produces a unique 256-bit "fingerprint" of a file. DokTUI uses SHA-256 checksums to verify that downloaded update binaries haven't been corrupted or tampered with.

**SSH (Secure Shell)**
> A protocol for securely connecting to remote computers over an encrypted channel. DokTUI uses SSH to send commands to your servers.

**State Machine**
> A model where a system can be in one of several "states" and transitions between them based on events. DokTUI's SSH connection uses a state machine: `Disconnected → Connecting → Connected → Reconnecting`.

---

### T

**TDD (Technical Design Document)**
> A document that describes *how* a product is built technically — the architecture, libraries, data flow, and implementation details. DokTUI's TDD is at `docs/TDD.md`.

**TEA** — See "Elm Architecture."

**Tick (Animation Tick)**
> A single "heartbeat" of the application loop. DokTUI has two tick rates: a fast one (~15 FPS) for smooth animations, and a slow one (1 second) for heavy operations like polling Docker status.

**TLS (Transport Layer Security)**
> The encryption protocol behind HTTPS. When DokTUI sets up Traefik with Let's Encrypt, it's enabling TLS so your apps are served securely.

**TOML (Tom's Obvious Minimal Language)**
> A configuration file format that's easy for humans to read and write. DokTUI uses TOML for config files and theme definitions.

**Traefik**
> A modern reverse proxy and load balancer that automatically discovers services via Docker labels. DokTUI uses Traefik to route web traffic (domains) to your deployed containers with automatic HTTPS.

**TUI (Terminal User Interface)**
> A graphical-style user interface that runs inside a terminal/console, using text characters, colors, and Unicode symbols instead of traditional GUI windows.

---

### U

**Unicode**
> A standard that defines characters for every writing system in the world, plus symbols. DokTUI uses Unicode block characters (like `█`, `▀`, `●`) to create its pixel-art visual style.

---

### V

**VPS (Virtual Private Server)**
> A virtual machine you rent from a cloud provider (like DigitalOcean, Hetzner, or AWS). DokTUI is designed to manage these.

---

### W

**Workspace (Cargo Workspace)**
> A Rust project structure that can contain multiple related packages. DokTUI is currently a single-crate workspace.

---

### Y

**YAML (YAML Ain't Markup Language)**
> A human-readable data format commonly used for configuration. Docker Compose files (`docker-compose.yml`) are written in YAML.

---

## Related Documentation

| Document | Description |
|----------|-------------|
| [PRD.md](./PRD.md) | Product requirements, goals, user personas, and feature scope |
| [TDD.md](./TDD.md) | Technical architecture, module structure, concurrency model, SSH layer |
| [DESIGN.md](./DESIGN.md) | Visual language, animation system, modular theme architecture |
| [TRAEFIK-ROUTING.md](./TRAEFIK-ROUTING.md) | How Traefik routing works — label generation, shared networks |
| [LAYOUT-REVISION.md](./LAYOUT-REVISION.md) | Onboarding screen layout, responsive/compact mode |
| [INTERACTION-AND-POLISH.md](./INTERACTION-AND-POLISH.md) | Mouse support, animation timing, pixel design components |

---

*This guide is a living document. If you find something unclear or missing, improving this guide is itself a great contribution!*
