# LabForge

A local-first desktop app for practicing cybersecurity skills and documenting findings.
It runs offline, keeps everything in a local SQLite database, and only ever works against
authorized, isolated training material.

**Status: Milestone 1 (desktop foundation).** One bundled exercise ("Read a DNS timeout and
retry") with guided steps, hints, a knowledge check, per-attempt progress, and an
attempt-linked Markdown notebook. No Docker, no accounts, no network access.
See [docs/ROADMAP.md](docs/ROADMAP.md) for what comes next.

Stack: Tauri 2, Rust, React 19 + Vite + TypeScript, Tailwind CSS 4, SQLite via SQLx, pnpm.

## Prerequisites

Ubuntu is the primary target; Windows is second and **not yet verified** (see the roadmap).

### Ubuntu

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Then install Rust ([rustup](https://rustup.rs), 1.85 or newer), Node.js 20+ and pnpm
(`corepack enable` or `npm i -g pnpm`). These are Tauri's documented Linux prerequisites;
the WebKitGTK 4.1 package is what the native window needs.

### Windows (unverified)

Install the Microsoft C++ Build Tools ("Desktop development with C++"), make sure the
WebView2 runtime is present (it ships with Windows 11), then Rust with the MSVC toolchain,
Node.js 20+ and pnpm. See <https://v2.tauri.app/start/prerequisites/>.

## Run it

```bash
pnpm install
pnpm tauri dev          # native window with hot reload
```

Build a production binary (no installer):

```bash
pnpm tauri build --no-bundle    # binary at src-tauri/target/release/labforge
```

Your data lives in the platform app data directory (on Ubuntu,
`~/.local/share/app.labforge.desktop/labforge.db`). To use a throwaway location, for example
to test a fresh start, set `LABFORGE_DATA_DIR`:

```bash
LABFORGE_DATA_DIR=/tmp/labforge-scratch pnpm tauri dev
```

## Checks

```bash
pnpm typecheck                              # TypeScript
pnpm test                                   # frontend tests (Vitest + Testing Library)
pnpm build                                  # type check + production frontend build
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                                  # unit + database tests against real SQLite files
```

## Try the restart check yourself

1. `pnpm tauri dev`, open **Challenges**, and start the exercise.
2. Complete step 1, write something in the notebook, and wait for "Saved".
3. Quit the app completely (close the window), then run `pnpm tauri dev` again.
4. **Dashboard** shows "Continue your attempt" with 1 of 4 steps done. Press **Resume**:
   step 2 is current and your notebook text is back.
5. Finish the attempt, then choose **Start another attempt**.
6. Open **History**: both attempts are listed, and the first one still opens with its notes.

## Layout

```
src/                React UI: api/ (typed IPC), views/, components/, tests next to code
src-tauri/
  migrations/       versioned SQLx migrations
  content/          bundled exercise definitions (JSON, under version control)
  capabilities/     which window may call which command
  src/              commands -> services -> repositories -> database (+ models, content)
docs/               ARCHITECTURE.md, SECURITY.md, ROADMAP.md, and the original design inputs
```

## Safety

LabForge is for authorized training only. It contains no attack automation, no terminal,
and no way for the UI to run arbitrary commands. The rules future lab features must meet
are in [docs/SECURITY.md](docs/SECURITY.md).
