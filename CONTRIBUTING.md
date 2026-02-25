# Contributing to Hagitori

Thank you for your interest in contributing to Hagitori! This guide covers everything you need to get started.

## Table of Contents

- [Development Environment](#development-environment)
- [Project Structure](#project-structure)
- [Code Style](#code-style)
- [Making Changes](#making-changes)
- [Pull Request Process](#pull-request-process)
- [Testing](#testing)
- [Reporting Bugs](#reporting-bugs)

---

## Development Environment

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | Edition 2024 | [rustup.rs](https://rustup.rs/) |
| Node.js | 22+ | [nodejs.org](https://nodejs.org/) |
| pnpm | Latest | `npm install -g pnpm` |
| Chrome/Chromium | Any | Required for browser-related features |

### Platform Setup

<details>
<summary><strong>Windows</strong></summary>

- Install [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/downloads/) with the **C++ build tools** workload
- Install Windows SDK 10/11
- Install Rust: `winget install Rustlang.Rustup`

</details>

<details>
<summary><strong>Linux (Debian/Ubuntu)</strong></summary>

```bash
sudo apt install -y build-essential pkg-config libssl-dev \
    libwebkit2gtk-4.1-dev librsvg2-dev libayatana-appindicator3-dev \
    patchelf libxdo-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

</details>

### Getting Started

```bash
git clone https://github.com/hagitori/hagitori.git
cd hagitori
pnpm install
pnpm tauri dev
```

---

## Project Structure

```
hagitori/
├── src/                    # Frontend (React + TypeScript)
├── src-tauri/              # Backend (Rust + Tauri v2)
│   ├── src/                #   App bootstrap and Tauri commands
│   └── crates/             #   9 internal crates (core, http, browser, ...)
└── .github/                # Workflows and scripts (bump-version, etc.)
```

Extensions live in a separate repository: [hagitori-extensions](https://github.com/hagitori/hagitori-extensions). See its CONTRIBUTING.md for extension development guidelines.

The Rust backend is organized as a Cargo workspace with 9 crates.

---

## Code Style

### Rust

- **Edition 2024** — use the latest language features
- **Clippy clean** — `cargo clippy --workspace --all-targets` must produce **zero warnings**
- **Workspace lints** — lint rules are centralized in the root `Cargo.toml` under `[workspace.lints.clippy]`; all crates inherit via `[lints] workspace = true`
- **`unsafe` is denied** — `unsafe_code = "deny"` is set at the workspace level
- **Error handling** — use `thiserror` for typed errors, never `unwrap()` in production code. Prefer `?` with `#[from]` conversions. Use `parking_lot` locks instead of `std::sync` to avoid poisoning and `.expect()` calls

### TypeScript / React

- **TypeScript strict mode** enabled
- **Functional components** with hooks
- **Zustand** for global state, **TanStack Query** for server state
- **Tailwind CSS** for styling — no inline styles or CSS modules

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add webtoon slicer support
fix: handle empty chapter list gracefully
refactor: simplify download engine retry logic
docs: update extension API reference
chore: bump tauri to v2.10
```

---

## Making Changes

### Workflow

1. **Fork** the repository and create a branch from `main`
2. **Make changes** — keep commits focused and atomic
3. **Test** — run `cargo test` and `cargo clippy --workspace --all-targets`
4. **Push** your branch and open a Pull Request

---

## Pull Request Process

1. Fill out the PR template with a clear description of what changed and why
2. Ensure all CI checks pass (clippy, tests, build)
3. Link related issues (e.g., `Closes #42`)
4. Keep PRs small and focused — one feature/fix per PR
5. Be responsive to review feedback

### Review Criteria

- Code follows the style guidelines above
- No new clippy warnings
- Public APIs have doc comments
- Tests cover the happy path and key edge cases
- No `unwrap()` or `.expect()` in production code paths

---

## Testing

### Rust Tests

```bash
cd src-tauri

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p hagitori-core

# Run a specific test
cargo test -p hagitori-download -- download_engine
```

### Clippy

```bash
cd src-tauri
cargo clippy --workspace --all-targets
```

Zero warnings is the hard requirement. The workspace has pre-configured lint levels — just follow the compiler's suggestions.

### Doc Tests

Doc examples in `///` comments are compiled and run as tests:

```bash
cd src-tauri
cargo test --doc
```

---

## Reporting Bugs

When opening a bug report, please include:

1. **OS and version** (e.g., Windows 11 23H2, Ubuntu 24.04)
2. **Hagitori version** (from Settings or `--version`)
3. **Steps to reproduce** — minimal, specific steps
4. **Expected vs actual behavior**
5. **Logs** — check the app's log output if applicable
6. **Extension info** — if the bug involves a specific extension, include its name and version

---

## Questions?

If something is unclear, open a [Discussion](https://github.com/hagitori/hagitori/discussions) or reach out in an issue. We're happy to help!
