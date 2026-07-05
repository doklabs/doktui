# PRD — DokTUI

**Status:** Draft
**Tanggal:** 6 Juli 2026
**Proyek:** Doklabs (open source)
**Versi:** 0.1

---

## 1. Ringkasan

DokTUI adalah produk **open source dari Doklabs** — pengganti [Dokploy](https://dokploy.com) dalam bentuk aplikasi **TUI (Terminal User Interface)** yang lebih ringkas dan efisien. Alih-alih menjalankan panel web yang berat di server, DokTUI berjalan **secara lokal** di mesin pengguna dan mengelola server remote lewat SSH.

Filosofi utama: **keep it simple**. Pengguna cukup meng-install dari repo publik, menjalankan DokTUI di lokal, mendaftarkan server SSH, lalu langsung mengelola deployment dari terminal — dengan sentuhan tampilan yang lebih interaktif dan gamified.

---

## 2. Latar Belakang & Masalah

Dokploy adalah PaaS self-hosted yang bagus, tetapi:

- Menuntut resource server untuk menjalankan dashboard web-nya sendiri (Postgres, Redis, UI, dsb.), yang memakan RAM/CPU pada server produksi.
- Overhead operasional: harus menjaga panel tetap hidup, ter-update, dan aman diakses.
- Untuk pengguna yang hanya mengelola beberapa server, ini terasa berlebihan.

DokTUI memindahkan "otak" kontrol ke sisi lokal pengguna. Server hanya perlu menjalankan Docker + Traefik. Tidak ada dashboard yang berjalan permanen di server, sehingga resource server sepenuhnya untuk aplikasi produksi.

---

## 3. Tujuan (Objective Goals)

- **Tech stack Rust** untuk membangun TUI (mengejar performa, binary tunggal, footprint kecil).
- **Fungsionalitas mirip Dokploy**: deploy aplikasi, manajemen container, environment variables, domain/routing via Traefik, logs, dan monitoring dasar.
- **Menghilangkan fitur yang tidak relevan** karena DokTUI berjalan lokal (mis. sistem multi-user berbasis web, auth panel, billing, manajemen tim berbasis server).
- **Canvas code editor built-in** dengan dua mode: **Vim** dan **non-Vim**, untuk mengedit config/compose/env langsung dari TUI.
- **Lintas platform**: berjalan di **macOS, Linux, dan Windows**, sehingga semua pengguna dapat merasakan manfaatnya.

## 4. Bukan Tujuan (Non-Objective Goals)

- **Auto-reconnect SSH**: koneksi remote SSH harus melakukan reconnecting otomatis agar sesi tidak terasa putus-putus; perilaku ini aktif **secara default**.
- **Kesederhanaan penggunaan**: pengalaman harus tetap sesederhana mungkin.
- **Tampilan gamified & interaktif**: UI boleh lebih hidup, dengan elemen gamifikasi dan interaktivitas.

> Catatan: item pada bagian ini adalah properti/kualitas produk yang diinginkan namun bukan "fitur fungsional inti" — melainkan prinsip desain dan perilaku default yang harus dipenuhi implementasi.

---

## 5. Persona Pengguna

- **Solo developer / indie hacker** yang mengelola 1–5 VPS dan ingin deploy cepat tanpa panel berat.
- **DevOps kecil / tim ramping** yang nyaman di terminal dan menghargai workflow keyboard-driven.
- **Pengguna eks-Dokploy** yang ingin alternatif lebih ringan namun familiar.

---

## 6. Alur Onboarding

Alur utama dirancang minimal dan linier:

1. **Instalasi** — pengguna meng-install DokTUI **langsung tanpa perlu cargo/toolchain Rust**. Metode utama: skrip satu baris (`curl -fsSL … | sh` untuk macOS/Linux, `irm … | iex` / installer `.exe` atau `winget`/`scoop` untuk Windows) yang mengunduh **prebuilt binary** dari release repo publik. Skrip otomatis mendeteksi OS & arsitektur dan mengambil binary yang sesuai.
2. **Menjalankan lokal** — DokTUI dijalankan di mesin lokal pengguna (`doktui`).
3. **Registrasi remote SSH** — pengguna diarahkan untuk mendaftarkan koneksi SSH server mereka (host, user, port, key/credential).
4. **Pengecekan server** — DokTUI memeriksa apakah server sudah terpasang **Docker** dan **Traefik**.
   - **Jika belum** → DokTUI melakukan **instalasi secara remote** (install Docker + Traefik lewat SSH).
   - **Jika sudah** → pengguna langsung diarahkan ke **dashboard DokTUI**.
5. **Dashboard** — pengguna mulai mengelola deployment.

```
Install (repo publik)
        │
        ▼
Jalankan DokTUI di lokal
        │
        ▼
Daftarkan remote SSH
        │
        ▼
Cek Docker + Traefik di server ──── sudah? ────► Dashboard DokTUI
        │
       belum
        │
        ▼
Install Docker + Traefik via remote ───────────► Dashboard DokTUI
```

---

## 7. Fitur & Ruang Lingkup

### 7.1 Fitur Inti (mirip Dokploy)

- **Manajemen server** — daftar server SSH, status koneksi, health check.
- **Deploy aplikasi** — dari Git repo, Docker image, atau Docker Compose.
- **Manajemen container** — start/stop/restart/remove, lihat status.
- **Environment variables & secrets** — kelola per aplikasi.
- **Domain & routing** — integrasi Traefik untuk domain, subdomain, dan TLS otomatis (Let's Encrypt).
- **Logs** — streaming log container secara real-time di TUI.
- **Monitoring dasar** — CPU, memori, dan status container per server.

### 7.2 Canvas Code Editor

- Editor terintegrasi untuk mengedit file config, `docker-compose.yml`, `.env`, dan Dockerfile langsung dari TUI.
- **Mode Vim** dan **mode non-Vim** (dapat dipilih pengguna).
- **Syntax highlighting** untuk YAML, TOML, ENV, Dockerfile, dan JSON — tersedia sejak rilis awal.

### 7.3 Perilaku Koneksi

- **Auto-reconnect SSH** aktif secara default; sesi remote terasa mulus meski jaringan tidak stabil.
- Indikator status koneksi yang jelas di UI.

### 7.4 Pengalaman & Tampilan

- Gamifikasi **dibatasi hanya pada karakter/aspek visual UI** — mis. maskot/karakter, ikonografi, warna, dan animasi ringan pada elemen antarmuka. **Tidak** ada sistem poin, level, reward, atau achievement fungsional yang mempengaruhi workflow.
- Navigasi keyboard-driven, shortcut yang konsisten dan mudah diingat.
- Konsisten di seluruh platform (macOS, Linux, Windows).

### 7.5 Update Binary (Manual)

- **Update bersifat manual**, tidak ada auto-update senyap. Ini disengaja: DokTUI mengelola server produksi, jadi perubahan binary yang tiba-tiba saat sedang deploy berisiko dan mengurangi prediktabilitas.
- **Notify-on-launch**: saat dijalankan, DokTUI mengecek versi terbaru secara async di background (non-blocking, tidak menahan startup) dan menampilkan notice kecil jika ada versi baru — mis. "v0.3 tersedia — jalankan `doktui update`".
- **Perintah `doktui update`**: mengunduh binary rilis sesuai OS/arsitektur, memverifikasi integritas, mengganti binary in-place, dan menampilkan changelog singkat. Hanya berjalan atas perintah eksplisit pengguna.
- **Deteksi metode instalasi**: jika DokTUI di-install lewat package manager (Homebrew/winget/scoop/AUR), `doktui update` mundur dan mengarahkan pengguna ke manajer paket (`brew upgrade`, dll.) agar versi tidak bentrok. Self-update in-place hanya aktif untuk instalasi via skrip langsung.
- **Opt-out**: pengecekan versi dapat dimatikan sepenuhnya lewat config, untuk lingkungan air-gapped atau pengguna yang tidak ingin ada koneksi keluar saat startup.

### 7.6 Di Luar Ruang Lingkup (Fitur yang Dihapus)

Karena DokTUI berjalan lokal, fitur berikut **tidak** disediakan:

- Panel web / dashboard berbasis browser di server.
- Sistem multi-user & manajemen tim berbasis server.
- Autentikasi/login panel berbasis web.
- Billing / subscription.
- Layanan background yang berjalan permanen di server (selain Docker & Traefik milik pengguna).

---

## 8. Arsitektur Teknis (High-Level)

- **Bahasa:** Rust.
- **Framework TUI:** kandidat `ratatui` + `crossterm` (final saat desain teknis).
- **SSH:** library SSH Rust (mis. `russh` / `ssh2`) dengan lapisan auto-reconnect di atasnya.
- **Runtime async:** `tokio` untuk menangani koneksi remote & streaming log secara konkuren.
- **Editor:** komponen editor kustom / crate yang mendukung mode Vim & non-Vim.
- **Konfigurasi lokal:** disimpan di mesin pengguna (mis. `~/.config/doktui/` di macOS/Linux, `%APPDATA%\doktui\` di Windows), tanpa server state.
- **Lintas platform & multi-arsitektur:** prebuilt binary tersedia untuk macOS, Linux, dan Windows pada arsitektur **amd64 (x86_64)** dan **arm64 (aarch64)**. Rilis dibangun lewat CI cross-compilation, tanpa mengharuskan pengguna memiliki toolchain Rust. `crossterm` dipilih karena portabilitas terminal lintas OS; path config & handling terminal disesuaikan per platform.
- **SSH key:** DokTUI **meng-generate key khusus DokTUI** (dedicated keypair) saat onboarding, terpisah dari key sistem pengguna, untuk isolasi dan kemudahan pencabutan akses.
- **Model kontrol:** semua orkestrasi dijalankan dari lokal → perintah ke server via SSH → server hanya menjalankan Docker + Traefik.

```
┌────────────────────────────┐        SSH (auto-reconnect)        ┌──────────────────────┐
│   DokTUI (lokal, Rust)     │ ─────────────────────────────────► │   Server Remote      │
│  - TUI (ratatui)           │                                    │   - Docker           │
│  - Editor (vim/non-vim)    │ ◄───────── logs / status ───────── │   - Traefik          │
│  - Config lokal            │                                    │   - Container app    │
└────────────────────────────┘                                    └──────────────────────┘
```

---

## 9. Keamanan

Karena DokTUI berjalan di **device lokal pengguna** dan memegang akses SSH ke server produksi, keamanan sisi lokal adalah prioritas utama. Binary ini adalah target bernilai tinggi — mengkompromikannya berarti mengompromikan seluruh server yang dikelola.

### 9.1 Penyimpanan Kredensial Lokal

- **SSH private key** disimpan dengan permission ketat (`0600` di macOS/Linux; ACL setara di Windows). DokTUI menolak berjalan jika permission file key terlalu longgar.
- Idealnya key diamankan lewat **OS keychain/secret store** bila tersedia (macOS Keychain, Windows Credential Manager, `libsecret`/Secret Service di Linux), dengan fallback ke file terenkripsi.
- **Opsi passphrase-protected key** dan integrasi **ssh-agent** untuk pengguna yang tidak ingin key tersimpan tanpa proteksi.
- **Secrets/env aplikasi** yang dikelola DokTUI tidak disimpan sebagai plaintext di config; dienkripsi saat at-rest di device lokal.

### 9.2 Integritas Binary & Update

- Setiap update memverifikasi **checksum SHA-256** dan, idealnya, **signature** rilis sebelum mengganti binary. Update ditolak jika verifikasi gagal.
- Skrip installer awal juga menyediakan checksum yang dapat diverifikasi, dan seluruh unduhan dilakukan lewat HTTPS.
- Build rilis dilakukan lewat CI yang reproducible sebisa mungkin, dengan artefak yang di-sign.

### 9.3 Prinsip Keamanan Lain

- **Transport aman**: semua komunikasi ke server lewat SSH terenkripsi; verifikasi host key (known_hosts) dengan peringatan jelas saat fingerprint berubah (mitigasi MITM).
- **Least privilege**: key khusus DokTUI memudahkan pencabutan akses tanpa mengganggu key sistem pengguna.
- **Tanpa telemetry diam-diam**: tidak ada pengiriman data pengguna. Satu-satunya koneksi keluar default adalah pengecekan versi, yang dapat di-opt-out (lihat 7.5).
- **Redaksi log**: secrets dan kredensial tidak ikut tertulis ke log atau tampilan UI.
- **Konfirmasi aksi destruktif**: operasi berisiko (hapus container, overwrite config) memerlukan konfirmasi eksplisit.

---

## 10. Metrik Keberhasilan

- **Time-to-first-deploy**: waktu dari install sampai deploy pertama sukses (target: semenit-menitan, bukan berjam-jam).
- **Footprint server**: 0 proses dashboard tambahan di server selain Docker/Traefik.
- **Stabilitas koneksi**: sesi SSH pulih otomatis tanpa intervensi pengguna pada gangguan jaringan singkat.
- **Kepuasan onboarding**: pengguna menyelesaikan alur onboarding tanpa dokumentasi eksternal.

---

## 11. Pertanyaan Terbuka

- Nama channel/kanal distribusi tambahan (mis. apakah perlu Homebrew tap, paket AUR, dsb.) di luar skrip installer utama.
- Pilihan mekanisme signature untuk rilis (mis. minisign/cosign) pada implementasi verifikasi update.

**Sudah diputuskan:**

- **Update binary** → manual (`doktui update`), notify-on-launch, verifikasi integritas, deteksi metode instalasi, dapat di-opt-out.
- **Instalasi** → langsung tanpa cargo; skrip installer mengunduh prebuilt binary dari release repo publik.
- **Multi-arsitektur** → amd64 (x86_64) dan arm64 (aarch64) didukung sejak rilis awal, lintas macOS/Linux/Windows.
- **Syntax highlighting editor** → tersedia sejak rilis awal (YAML, TOML, ENV, Dockerfile, JSON).
- **SSH key** → generate keypair khusus DokTUI (dedicated), terpisah dari key sistem pengguna.
- **Gamifikasi** → dibatasi hanya pada karakter/visual UI, tanpa mekanik reward fungsional.
- **Platform** → macOS, Linux, dan Windows didukung.

---

## 12. Rencana Rilis (Tentatif)

- **v0.1 (MVP):** onboarding, registrasi SSH, cek + install Docker/Traefik, deploy dasar, logs, auto-reconnect, penyimpanan key aman + verifikasi host key, update manual (`doktui update`).
- **v0.2:** canvas code editor (Vim & non-Vim) dengan syntax highlighting, manajemen env/secrets terenkripsi, monitoring dasar.
- **v0.3:** polish karakter/visual UI, peningkatan UX & shortcut, integrasi keychain OS penuh.
