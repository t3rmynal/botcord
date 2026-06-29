# botcord

A local desktop toolkit for managing multiple Discord accounts. Runs entirely on your machine — no servers, no cloud, no middleman.

Built with [Tauri](https://tauri.app) (Rust backend) and React + TypeScript frontend. Tokens are stored encrypted in a local SQLite database, locked behind a master password.

**Platform:** Windows 11 only.

---

## Features

- **Account manager** — import tokens, validate them in bulk, view profile info (avatar, display name, Nitro status)
- **Token checker** — paste a list of tokens and instantly see which ones are valid
- **Proxy support** — add HTTP/SOCKS proxies, run health checks, auto-assign to accounts
- **Voice presence** — join voice channels across multiple accounts simultaneously
- **DM broadcast** — send a message (text or image) to all friends of selected accounts with configurable rate limiting
- **Account registration** — register fresh accounts, auto-handles captcha via an embedded Chromium session
- **Invite & friend tools** — bulk-accept invites, send friend requests across accounts
- **Profile editing** — change username, avatar, or other profile fields on selected accounts
- **Isolated browser sessions** — each account gets its own Chromium profile for login flows
- **Encrypted vault** — all tokens are AES-encrypted at rest; nothing leaves your machine

---

## Requirements

- Windows 11
- [Node.js](https://nodejs.org) 20+
- [pnpm](https://pnpm.io) 9+
- [Rust](https://rustup.rs) 1.80+

---

## Getting started

```bash
pnpm install
pnpm run setup      # fetches Chromium and browser extensions
pnpm tauri dev
```

`pnpm run setup` downloads ungoogled-Chromium portable and Privacy Badger into the correct locations under `src-tauri/resources/`. It only needs to run once.

---

## Build

```bash
pnpm tauri build
```

Produces an NSIS installer in `src-tauri/target/release/bundle/nsis/`.

For a standalone portable build (no installer):

```bash
pnpm run portable
```

---

## Data locations

| What | Path |
|------|------|
| Database | `%APPDATA%\botcord\botcord.sqlite` |
| Chromium profiles | `%APPDATA%\botcord\profiles\<account_id>\` |

The database is created on first launch. Your master password is never stored — it is used to derive the encryption key for all token fields.

---

## Project layout

```
botcord/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── routes/             # Page-level views (Accounts, Proxies, Servers, Inbox)
│   └── store/              # Zustand state
├── src-tauri/
│   ├── src/
│   │   ├── commands/       # Tauri IPC handlers (accounts, proxies, voice, broadcast, …)
│   │   ├── discord/        # Discord HTTP client and API wrappers
│   │   └── storage/        # SQLite + AES-GCM encryption
│   └── sidecars/voice/     # Node.js voice gateway sidecar
└── scripts/
    ├── fetch-resources.mjs # Downloads Chromium + extensions
    └── make-portable.mjs   # Packages a portable build
```

---

## Tech stack

| Layer | What |
|-------|------|
| Shell | Tauri 2 |
| Backend | Rust — `rusqlite`, `reqwest`, `tokio`, `aes-gcm` |
| Frontend | React 18, TypeScript, Tailwind CSS 4, Zustand, Framer Motion |
| Build | Vite + `@tauri-apps/cli` |
| Voice | Node.js sidecar (`@discordjs/voice`) |

---

## License

MIT
