# Security

LabForge is a training tool. It must only ever be pointed at authorized, isolated, local
training material, and it must never become a general-purpose attack or automation tool.

## Trust boundaries

1. **WebView to Rust.** The WebView is treated as untrusted input. Rust validates every
   argument; the frontend never supplies SQL, shell commands, file paths, or container arguments.
2. **Rust to SQLite.** All queries are parameterized string literals. User text (such as the
   challenge search) is bound as a parameter, and `LIKE` wildcards in it are escaped.
3. **Rust to the system browser.** One command opens links, and it accepts only plain `https` URLs.
4. **Rust to Docker (Milestone 3, not built).** The daemon is a privileged dependency. See below.

## Controls in place (Milestone 1)

| Control | Where |
| --- | --- |
| Command allow-list: only commands listed in `build.rs` exist, each needs an explicit `allow-*` permission | `src-tauri/build.rs`, `capabilities/default.json` |
| Permissions scoped to the `main` window; no core, plugin, filesystem, shell or HTTP permissions granted | `capabilities/default.json` |
| Restrictive CSP: `default-src 'self'`, no inline scripts or styles, no remote origins, `object-src 'none'`, `frame-ancestors 'none'` | `tauri.conf.json` |
| Dev-only CSP relaxation is a separate `devCsp`; production uses `csp` | `tauri.conf.json` |
| Input validation in Rust: lengths, key charset, step counts, choice rules, note size | `services/validation.rs` |
| SQL `CHECK` constraints mirror the limits as a backstop | `migrations/0001_init.sql` |
| Markdown renders with raw HTML disabled; images are not loaded; links never navigate the WebView | `components/Markdown.tsx` |
| External links: `https` only, no credentials, no whitespace or control characters, length-capped; validated again in Rust | `services/links.rs` |
| Internal errors are logged, not returned; the UI gets a generic message | `error.rs` |
| Correct answers to knowledge checks never leave Rust | `repositories/challenges.rs` |
| The opener plugin is registered but not granted to the WebView; only our validated command uses it | `lib.rs`, `capabilities/default.json` |
| `unsafe` code is forbidden in the crate | `Cargo.toml` |
| No secrets or credentials are bundled; there is no account, cloud service, or network access | whole app |
| Bundled content uses fictional hosts on private addresses and the reserved `.test` domain | `content/` |

## Not in scope, deliberately

No Docker orchestration, no execution of real offensive tools, no unrestricted terminal, no
authentication, no cloud services. Milestone 1 contains no container code at all.

## Requirements for the Docker runtime (Milestone 3)

These are the acceptance conditions for the lab runtime, from the project prompt and the design
report. Nothing ships until each is met and tested.

- **Templates only.** Docker actions come from validated, approved lab templates identified by
  known IDs. The frontend never sends shell commands, image names, or Docker arguments.
- **Fixed configuration per template:** digest-pinned image, a dedicated network, CPU and
  memory limits, and its own data volumes only.
- **Unprivileged.** No privileged mode, no host networking, and never mount the Docker socket
  into a lab container.
- **Loopback by default.** Published ports bind to `127.0.0.1`. After launch, verify the real
  port bindings, confirm the host can reach the fixture, and confirm a second LAN device cannot.
- **No outbound network** for the pilot.
- **Docker version check.** Docker documents a localhost-publishing exception for releases older
  than 28.0.0. Check the engine version and test network isolation before enabling any
  vulnerable training image.
- **Reconcile, don't trust.** Track instances by runtime ID and an ownership label. On launch and
  after each operation, read actual engine state; report Docker as unavailable if it can't be read.
- **Scoped destructive actions.** Stop and reset apply only to app-owned instances and their
  designated data, require confirmation, and preserve notes and attempt history.
- **Recoverable failures:** missing Docker permissions, a busy port, and an interrupted startup
  each show a clear status and a next action.
- OWASP Juice Shop is evaluated only after a benign local HTTP fixture passes these checks.

## Reviewing a change

Ask of every diff: what does it expose, what does it trust, what can leak, and is the default
closed? A new command must be added to `build.rs` and granted explicitly. A new permission in
`capabilities/` needs a reason written next to it. Anything touching `tauri.conf.json` security
settings needs the same scrutiny as code.

## Known gaps

- The Ubuntu and Windows WebViews enforce CSP slightly differently; only Ubuntu has been exercised.
- Notes are stored unencrypted in the local database. The threat model assumes a single-user
  desktop where the learner owns the files.
- Dependencies are not yet audited automatically (`cargo audit` / `pnpm audit` are not wired into CI).
