# TDD — DokTUI

**Status:** Draft
**Tanggal:** 6 Juli 2026
**Proyek:** Doklabs (open source)
**Versi:** 0.1
**Referensi:** [PRD.md](./PRD.md)

---

## 1. Tujuan Dokumen

Dokumen ini menjabarkan desain teknis DokTUI: bagaimana komponen-komponen diimplementasikan untuk memenuhi kebutuhan pada PRD. Fokusnya pada arsitektur modul, pilihan library, alur data, model konkurensi, layer SSH, editor, keamanan, dan pipeline build/rilis. Detail UI visual berada di luar cakupan dokumen ini kecuali yang berdampak pada arsitektur.

---

## 2. Ringkasan Arsitektur

DokTUI adalah aplikasi TUI **single-binary** berbasis Rust yang berjalan di device lokal pengguna. Semua orkestrasi dilakukan dari lokal; server remote hanya menjalankan Docker + Traefik dan menerima perintah lewat SSH.

Arsitektur mengikuti pola **berlapis (layered)** dengan pemisahan jelas antara UI, state/logika aplikasi, dan I/O (SSH, filesystem, jaringan).

```
┌──────────────────────────────────────────────────────────────┐
│                        DokTUI (lokal)                         │
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
                                             │ SSH (terenkripsi)
                                             ▼
                              ┌────────────────────────────┐
                              │   Server Remote             │
                              │   Docker + Traefik          │
                              └────────────────────────────┘
```

---

## 3. Tech Stack

| Kebutuhan | Pilihan | Alasan |
|---|---|---|
| Bahasa | Rust (edition 2021) | Performa, binary tunggal, footprint kecil, memory-safe |
| TUI framework | `ratatui` + `crossterm` | Portabilitas terminal lintas OS (macOS/Linux/Windows) |
| Async runtime | `tokio` | Konkurensi untuk banyak koneksi SSH & streaming log |
| SSH | `russh` (pure-Rust) | Tanpa dependensi C, kontrol penuh untuk auto-reconnect; kandidat alternatif `ssh2` (libssh2) |
| Editor buffer | `ropey` (rope data structure) | Efisien untuk edit teks besar |
| Syntax highlighting | `syntect` atau `tree-sitter` | Highlighting YAML/TOML/ENV/Dockerfile/JSON |
| Serialisasi config | `serde` + `toml` | Config lokal berbasis TOML |
| Enkripsi at-rest | `age` / `chacha20poly1305` | Enkripsi secrets & key di device lokal |
| OS keychain | `keyring` crate | Akses Keychain/Credential Manager/Secret Service |
| Self-update | `self_update` (dimodifikasi) | Update manual dengan verifikasi checksum/signature |
| Verifikasi signature | `minisign` / `cosign` (TBD) | Integritas rilis |
| CLI args | `clap` | Parsing argumen (`doktui update`, dll.) |
| Logging | `tracing` + `tracing-subscriber` | Structured logging dengan redaksi secret |

---

## 4. Struktur Modul

Struktur crate (workspace tunggal, dipecah jadi beberapa modul):

```
doktui/
├── src/
│   ├── main.rs               # entrypoint, parsing CLI, bootstrap runtime
│   ├── app/
│   │   ├── mod.rs            # Application core, event loop
│   │   ├── state.rs         # AppState, model data terpusat
│   │   ├── event.rs         # definisi Event & Message
│   │   └── command.rs       # Command dispatcher (async)
│   ├── ui/
│   │   ├── mod.rs           # root render
│   │   ├── views/           # dashboard, server list, logs, dll.
│   │   ├── editor/          # canvas editor (vim & non-vim)
│   │   └── theme.rs         # karakter/visual UI (gamifikasi)
│   ├── services/
│   │   ├── ssh/             # SSH manager + auto-reconnect
│   │   ├── docker.rs        # perintah Docker via SSH
│   │   ├── traefik.rs       # provisioning Traefik
│   │   ├── provision.rs     # cek & install Docker/Traefik remote
│   │   ├── secrets.rs       # enkripsi/dekripsi secrets
│   │   └── updater.rs       # update manual binary
│   ├── config/
│   │   ├── mod.rs           # load/save config lokal
│   │   └── paths.rs         # path per-OS
│   └── security/
│       ├── keys.rs          # generate & simpan SSH key khusus DokTUI
│       ├── keychain.rs      # integrasi OS keychain
│       └── hostkey.rs       # verifikasi known_hosts
├── build.rs
└── Cargo.toml
```

---

## 5. Model Konkurensi & Event Loop

DokTUI memakai arsitektur **message-passing** ala Elm/TEA (The Elm Architecture) di atas `tokio`.

- **Main loop** (single-threaded pada UI) menangani input terminal dan render.
- **Background tasks** (`tokio` tasks) menangani I/O: koneksi SSH, streaming log, pengecekan versi, provisioning. Mereka berkomunikasi ke core lewat `tokio::sync::mpsc` channel.
- Setiap peristiwa (keypress, hasil SSH, tick timer) menjadi `Message` yang masuk ke antrian; core mengolahnya, memperbarui `AppState`, lalu memicu re-render.

```
[terminal input] ─┐
[ssh events]      ├──► mpsc channel ──► Core.update(Message) ──► AppState ──► UI.render()
[timer ticks]     ┘
```

Keuntungan: render tidak pernah blocking oleh I/O jaringan, sehingga UI tetap responsif (mendukung tujuan "tidak terasa putus-putus").

---

## 6. Layer SSH & Auto-Reconnect

Ini komponen paling kritikal untuk UX. Desain:

- **Connection pool**: satu koneksi persisten per server terdaftar, dikelola oleh `SshManager`.
- **State machine koneksi**: `Disconnected → Connecting → Connected → Reconnecting`. Transisi dipublikasikan ke UI sebagai indikator status.
- **Auto-reconnect (default aktif)**: saat koneksi putus, manager otomatis mencoba reconnect dengan **exponential backoff + jitter** (mis. 1s, 2s, 4s, … hingga batas maksimum), tanpa intervensi pengguna.
- **Keep-alive**: kirim SSH keep-alive/heartbeat periodik untuk mendeteksi koneksi mati lebih cepat.
- **Command queue**: perintah yang dikirim saat koneksi sedang down di-queue (atau ditandai gagal dengan retry) sehingga sesi terasa mulus.
- **Multiplexing**: gunakan channel SSH terpisah untuk streaming log vs. eksekusi perintah, agar log real-time tidak memblok perintah interaktif.

Verifikasi host key dilakukan lewat modul `security/hostkey.rs` terhadap `known_hosts` lokal; perubahan fingerprint memicu peringatan (mitigasi MITM).

---

## 7. Provisioning Remote (Cek & Install Docker/Traefik)

Alur pada `services/provision.rs`:

1. Setelah SSH terhubung, jalankan probe: `command -v docker`, `docker compose version`, dan cek container/servis Traefik.
2. Jika **belum ada**:
   - Deteksi OS/distro server (`/etc/os-release`).
   - Jalankan instalasi Docker (mis. script resmi `get.docker.com`) via SSH.
   - Deploy Traefik sebagai container dengan konfigurasi default (entrypoints, provider Docker, resolver TLS Let's Encrypt).
   - Verifikasi hasil instalasi.
3. Jika **sudah ada** → lanjut ke dashboard.

Setiap langkah menampilkan progress di UI; kegagalan menampilkan pesan actionable, bukan stack trace mentah.

---

## 8. Deploy & Manajemen Container

- **Sumber deploy**: Git repo (clone/pull di server), Docker image (pull), atau Docker Compose (upload/sinkron compose file).
- **Eksekusi**: DokTUI menyusun perintah `docker`/`docker compose` dan mengirimnya via SSH; output di-stream balik.
- **Routing Traefik**: label Traefik disuntikkan ke container/compose (host rule, entrypoint, TLS) berdasarkan domain yang dikonfigurasi pengguna.
- **Env & secrets**: di-inject saat runtime; nilai sensitif tidak ditulis plaintext ke server config maupun log.
- **Logs**: streaming `docker logs -f` lewat channel SSH khusus.

---

## 9. Canvas Code Editor

- **Buffer**: struktur `ropey` untuk edit efisien.
- **Mode**: `Vim` (modal: normal/insert/visual, subset motion & command umum) dan `non-Vim` (editing standar). Mode dipilih di config, dapat diganti runtime.
- **Syntax highlighting**: `syntect` (atau `tree-sitter`) untuk YAML, TOML, ENV, Dockerfile, JSON. Highlighting dijalankan incremental agar tidak memblok render.
- **File target**: config, `docker-compose.yml`, `.env`, Dockerfile — bisa file lokal maupun file remote yang di-fetch via SSH lalu di-sync balik.
- **Keamanan**: saat mengedit file berisi secret, editor menghormati aturan redaksi log dan tidak menuliskan buffer ke lokasi temporer tanpa proteksi.

---

## 10. Konfigurasi & Penyimpanan Lokal

- **Lokasi config**: `~/.config/doktui/` (macOS/Linux), `%APPDATA%\doktui\` (Windows) — di-resolve oleh `config/paths.rs` (crate `directories`).
- **Format**: TOML via `serde`.
- **Isi**: daftar server, preferensi editor (vim/non-vim), tema UI, flag opt-out update.
- **Secrets & key**: TIDAK disimpan plaintext. Private key khusus DokTUI dan secrets aplikasi dienkripsi at-rest (`age`/`chacha20poly1305`), dengan opsi menyimpan di OS keychain via crate `keyring`.
- **Permission**: file key dipaksa `0600` (macOS/Linux) / ACL setara (Windows); DokTUI menolak jalan jika permission terlalu longgar.

---

## 11. Keamanan (Implementasi)

Mengacu pada §9 PRD, implementasi teknisnya:

- **Generate key khusus DokTUI** saat onboarding (`security/keys.rs`) — Ed25519 keypair, terpisah dari key sistem pengguna, mudah dicabut.
- **Penyimpanan**: prioritas OS keychain; fallback file terenkripsi. Dukungan passphrase-protected key & integrasi `ssh-agent`.
- **Integritas update**: verifikasi SHA-256 + signature (minisign/cosign) sebelum swap binary; tolak jika gagal. Semua unduhan HTTPS.
- **Host key verification**: `known_hosts` lokal, peringatan saat fingerprint berubah.
- **Redaksi log**: layer `tracing` menyaring secret sebelum ditulis.
- **Tanpa telemetry**: satu-satunya koneksi keluar default adalah pengecekan versi (opt-out tersedia).
- **Konfirmasi aksi destruktif**: hapus container / overwrite config butuh konfirmasi eksplisit.

---

## 12. Updater (Manual)

Modul `services/updater.rs`:

- **Notify-on-launch**: task async mengecek endpoint rilis (mis. GitHub Releases API) saat startup, non-blocking; tampilkan notice bila ada versi baru.
- **`doktui update`**: unduh binary sesuai OS/arsitektur → verifikasi checksum & signature → swap in-place (atomic rename) → tampilkan changelog.
- **Deteksi metode instalasi**: baca marker instalasi; jika via package manager (Homebrew/winget/scoop/AUR), arahkan ke manajer paket alih-alih self-update.
- **Opt-out**: flag config mematikan seluruh pengecekan versi.

---

## 13. Build, Rilis & Distribusi

- **Cross-compilation** lewat CI (mis. `cargo` + `cross` / target khusus) untuk matriks:
  - macOS: `x86_64-apple-darwin`, `aarch64-apple-darwin`
  - Linux: `x86_64-unknown-linux-gnu` (dan/atau `-musl`), `aarch64-unknown-linux-gnu`
  - Windows: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`
- **Artefak**: prebuilt binary + file checksum + signature per target, dipublikasikan ke release repo publik.
- **Installer**:
  - macOS/Linux: skrip `curl -fsSL … | sh` yang deteksi OS/arch, unduh binary, verifikasi checksum, taruh di PATH.
  - Windows: skrip `irm … | iex` / installer `.exe`, plus opsi `winget`/`scoop`.
- **Tanpa toolchain Rust** di sisi pengguna.

---

## 14. Penanganan Error & Ketahanan

- Error jaringan/SSH ditangani oleh state machine reconnect (§6), bukan crash.
- Kegagalan perintah remote ditampilkan dengan konteks (perintah, exit code, stderr terkurasi).
- Panik di background task tidak boleh menjatuhkan UI; task diisolasi dan dilaporkan sebagai error state.
- Config corrupt → fallback ke default dengan peringatan, bukan gagal total.

---

## 15. Strategi Pengujian

- **Unit test**: parser probe, penyusunan perintah Docker/Traefik, logika backoff reconnect, enkripsi/dekripsi secrets.
- **Integration test**: SSH manager terhadap server Docker uji (container SSHD) — termasuk skenario koneksi putus untuk memvalidasi auto-reconnect.
- **Editor test**: operasi buffer (`ropey`), mode Vim motion, highlighting.
- **Security test**: verifikasi checksum/signature update ditolak saat dimanipulasi; permission key enforcement; redaksi log.
- **Cross-platform CI**: jalankan test pada macOS, Linux, Windows.
- **Verifikasi rilis**: smoke test binary hasil cross-compile per target.

---

## 16. Risiko Teknis & Mitigasi

| Risiko | Dampak | Mitigasi |
|---|---|---|
| Perbedaan perilaku terminal lintas OS (khususnya Windows) | UI rusak/inkonsisten | Andalkan `crossterm`; CI test per platform; hindari fitur terminal non-portabel |
| Auto-reconnect menutupi masalah jaringan nyata | Pengguna bingung | Indikator status jelas + log koneksi yang dapat diinspeksi |
| Kompleksitas mode Vim | Scope creep | Mulai dari subset motion/command umum, perluas bertahap |
| Keamanan penyimpanan key di device lokal | Kompromi server | OS keychain + enkripsi at-rest + permission enforcement |
| Instalasi Docker remote gagal di distro tak dikenal | Onboarding buntu | Deteksi OS + pesan fallback + dukungan skenario "sudah ada Docker" |

---

## 17. Pertanyaan Teknis Terbuka

- Final pilihan library SSH: `russh` (pure-Rust) vs `ssh2` (libssh2).
- `syntect` vs `tree-sitter` untuk highlighting (ukuran binary vs akurasi).
- Mekanisme signature rilis: `minisign` vs `cosign`.
- Apakah Linux musl static build dijadikan default untuk portabilitas maksimum.
