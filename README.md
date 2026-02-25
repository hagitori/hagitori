<p align="center">
  <img src="public/banner_bg.png" alt="Hagitori Banner" width="60%" />
</p>

<h1 align="center">
  Hagitori
</h1>

<p align="center">
  <strong>A fast, extensible manga downloader built with Rust</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-Edition_2024-b7410e?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/Tauri-v2-24C8D8?logo=tauri&logoColor=white" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black" alt="React" />
  <img src="https://img.shields.io/badge/TypeScript-5.8-3178C6?logo=typescript&logoColor=white" alt="TypeScript" />
  <img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT" />
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#getting-started">Getting Started</a> •
  <a href="#extensions">Extensions</a> •
  <a href="#contributing">Contributing</a> •
  <a href="#license">License</a>
</p>

---

## About

**Hagitori** is a cross-platform desktop manga downloader with a **TypeScript extension system** that lets the community write scrapers for any manga site. Built with **Rust + Tauri v2** on the backend and **React + TypeScript** on the frontend.

Inspired by [Pyneko](https://github.com/Lyem/Pyneko) and [HakuNeko](https://github.com/manga-download/hakuneko).

## Features

- **TypeScript Extension System** — Community-driven scrapers via sandboxed QuickJS runtime with typed SDK (HTTP, HTML parsing, browser control, crypto)
- **Parallel Downloads** — Concurrent image downloads with per-page retry and exponential backoff
- **Cloudflare Bypass** — Multi-layer bypass via Chrome DevTools Protocol (CDP)
- **CBZ/ZIP Packaging** — Auto-group chapters into CBZ, ZIP, or plain folders
- **Extension Sync** — GitHub-hosted catalog with auto-update and integrity checks
- **Real-time Progress** — Live download status via Tauri IPC events
- **Cross-platform** — Windows, Linux

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (Edition 2024)
- [Node.js](https://nodejs.org/) 22+ and [pnpm](https://pnpm.io/)
- Chrome or Chromium (for Cloudflare bypass)
- Platform build tools:
  - **Windows:** [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) (C++ workload)
  - **Linux (Debian/Ubuntu):**
    ```bash
    sudo apt install -y build-essential pkg-config libssl-dev \
      libwebkit2gtk-4.1-dev librsvg2-dev libayatana-appindicator3-dev \
      patchelf libxdo-dev
    ```

### Development

```bash
git clone https://github.com/hagitori/hagitori.git
cd hagitori
pnpm install
pnpm tauri dev
```

### Production Build

```bash
pnpm tauri:build
```

## Extensions

Extensions are written in **TypeScript**, transpiled to JavaScript via esbuild, and executed in a sandboxed QuickJS runtime. They implement three functions:

```typescript
function getManga(url: string): Manga { /* ... */ }
function getChapters(mangaId: string): Chapter[] { /* ... */ }
function getPages(chapter: Chapter): Pages { /* ... */ }
// Optional — provides additional manga details (synopsis, tags, status, etc.)
function getDetails(mangaId: string): MangaDetails { /* ... */ }
```

The SDK exposes built-in APIs: `http`, `html`, `browser`, `crypto`, `cookies`, `session`, and entity constructors.

> See the [hagitori-extensions](https://github.com/hagitori/hagitori-extensions) repository for the extension SDK, templates, and the full API reference.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, code style, and PR guidelines.

## License

[MIT License](hagitori-extensions/LICENSE)

---

<p align="center">
  <sub>Built with 🍃</sub>
</p>
