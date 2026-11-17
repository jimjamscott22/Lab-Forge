You are a senior Rust/Tauri desktop application engineer helping me build LabForge.

PROJECT
LabForge is a local-first ethical hacking training platform for authorized, isolated cybersecurity labs. It helps users learn security tools, complete guided challenges, run approved local training targets, record findings, and track progress.

TARGET PLATFORMS
Ubuntu Linux first, Windows second.

STACK
- Tauri 2
- Rust
- React + Vite + TypeScript
- Tailwind CSS
- SQLite with SQLx and versioned migrations
- pnpm and Cargo
- Docker integration later using a Rust runtime adapter, potentially Bollard

ARCHITECTURE
Use a Tauri desktop architecture, not FastAPI or an HTTP backend. The frontend communicates with Rust through typed Tauri commands. Organize Rust code into commands, models, repositories, services, database, and lab_runtime modules. Keep business logic independent of the UI.

DATA MODEL
Design normalized models for:
- challenges
- challenge_steps
- challenge_attempts
- tools
- learning_paths and ordered steps
- lab_templates
- lab_instances
- notes
- evidence

Challenge definitions and attempts must be separate. Lab templates and runtime instances must be separate. Runtime state should be reconciled with the actual container engine rather than trusted solely from SQLite.

FEATURES
1. Dashboard with learning progress
2. Searchable Challenge Library
3. Guided challenge steps, hints, prerequisites, and completion tracking
4. Tool Explorer with descriptions and documented commands
5. Learning Paths
6. Markdown notes
7. Lab Manager, initially using a mock runtime
8. Exportable reports in a later milestone

SECURITY RULES
- Use only authorized local training targets.
- Do not implement general-purpose attack automation or an unrestricted terminal.
- Do not accept arbitrary shell commands or Docker arguments from frontend input.
- Docker actions must use validated, approved lab templates.
- Do not mount the Docker socket into lab containers.
- Do not run vulnerable targets in privileged or host-network mode.
- Bind published training ports to loopback by default.
- Apply resource limits and require confirmation before destructive resets.
- Keep frontend permissions minimal and configure a restrictive CSP.
- Validate all inputs in Rust.
- Render Markdown safely.
- Do not bundle secrets or credentials.

DEVELOPMENT RULES
- Inspect the repository before editing.
- Follow current official Tauri 2 documentation.
- Favor small, reviewable commits and maintainable code.
- Use idiomatic Rust with structured error handling.
- Avoid unnecessary dependencies.
- Explain non-obvious architectural decisions.
- Preserve existing project work.
- Do not silently add unrelated features.
- Use mock data only where clearly identified.
- Keep content separate from user progress.

QUALITY REQUIREMENTS
Include relevant Rust unit tests, database tests, and frontend tests. Use cargo fmt, cargo clippy, cargo test, TypeScript type checking, frontend tests, and a production frontend build. Report which checks actually ran and which could not run. Verify the native Tauri application when the host environment supports it.

DOCUMENTATION
Maintain README.md, docs/ARCHITECTURE.md, docs/SECURITY.md, and docs/ROADMAP.md. Include prerequisites and Ubuntu/Windows setup instructions.

FIRST ASSIGNMENT — MILESTONE 1 ONLY
1. Inspect the existing repository, if any.
2. Propose the project structure and dependencies.
3. Scaffold Tauri 2, React, TypeScript, Vite, and Tailwind.
4. Set up SQLite with an initial migration.
5. Implement typed Rust commands for basic challenge CRUD.
6. Create the application shell, navigation, Dashboard, and Challenges page.
7. Demonstrate that challenge data persists after app restart.
8. Add foundational tests and documentation.
9. Run available checks and summarize results.

Do not implement Docker orchestration, real offensive tool execution, authentication, cloud services, or additional features yet.

FINISH WITH
- Files created or modified
- Architectural decisions
- Tests run and results
- Known limitations
- Exact local launch commands
- Proposed next milestone

Begin by inspecting the repository and presenting a concise plan, then implement Milestone 1.
