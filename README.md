# PDF Splitter

> A fast, beautiful cross-platform desktop app that splits any multi-page PDF into individual page files — built with Tauri 2 and Leptos (Rust → WASM).

![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8D8.svg)
![Leptos](https://img.shields.io/badge/Leptos-0.7-de4730.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey.svg)

---

## Overview

**PDF Splitter** is a cross-platform (macOS and Windows) desktop application that takes any multi-page PDF document and extracts every page into its own individual PDF file.

The app is built on a native [Tauri 2](https://tauri.app/) shell with a [Leptos 0.7](https://leptos.dev/) UI compiled to WebAssembly via [Trunk](https://trunk-rs.dev/), providing:

- **Native performance** — a lean Rust binary with zero Electron overhead.
- **Parallel page processing** — automatically scales to all available CPU cores for fast extraction.
- **Beautiful, native-feel UI** — glassmorphism design, dark-mode support, and smooth animations.
- **Drag & drop** — drop a PDF straight onto the window to quickly begin splitting.
- **100% Fidelity** — guarantees perfect quality for fonts, images, and embedded resources.

> The frontend was migrated from Vue 3 + TypeScript + Vite to Leptos + Trunk. See [`MIGRATION.md`](./MIGRATION.md) for the full rationale, procedure, and the runtime gotchas encountered (Tauri global API loading, reactive-signal disposal, CSP for WASM).

---

## Getting Started

### Prerequisites

- [Rust + Cargo](https://rustup.rs/) (≥ 1.80) with the `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- [Trunk](https://trunk-rs.dev/) (the WASM web bundler):
  ```bash
  cargo install trunk
  ```
- **macOS**: Xcode Command Line Tools (`xcode-select --install`)
- **Windows**: Build Tools for Visual Studio (C++ build tools)
- *(Optional, only to run the frontend unit tests)* — [Node.js](https://nodejs.org/) and `wasm-bindgen-cli`:
  ```bash
  cargo install wasm-bindgen-cli --version 0.2.126
  ```

### Development

```bash
# 1. Clone the repository
git clone https://github.com/suradet-ps/pdf-splitter.git
cd pdf-splitter

# 2. Start the app (hot-reload for both the Rust backend and the Leptos UI)
cargo tauri dev
```

`cargo tauri dev` runs `trunk serve` for the UI (hot-reload at http://127.0.0.1:1420) and launches the Tauri window.

To iterate on the **frontend only** (no Tauri window), run the Trunk dev server directly:

```bash
cd src
trunk serve        # serves the Leptos app at http://127.0.0.1:1420
```

### Production Build

```bash
# Build the optimized binary + platform installer (macOS .app/.dmg, Windows .msi/.exe)
cargo tauri build
```

The output artifacts are generated in `src-tauri/target/release/bundle/`.

*Note: an automated GitHub Actions workflow (`build-windows.yml`) builds and releases the Windows installer whenever a new `v*` tag is pushed. Its toolchain (Bun-based `tauri` CLI invocation) should be updated to the Cargo-based `cargo tauri build` described above.*

---

## Tech Stack

- **Backend**: Rust, Tauri 2, `lopdf` (PDF processing), `rayon` (parallel processing).
- **Frontend**: [Leptos 0.7](https://leptos.dev/) (client-side rendered), compiled to `wasm32-unknown-unknown` via [Trunk](https://trunk-rs.dev/). No JavaScript/TypeScript/bundler.

### Repository layout

```
crates/pdf-split-core/   # pure-Rust PDF engine (no Tauri dependency)
src-tauri/               # Tauri app shell + Rust commands (thin IPC layer)
src/                     # Leptos frontend crate (excluded from the Cargo workspace;
                         #   built & tested separately with trunk / wasm-bindgen-test)
```

The frontend crate is intentionally **excluded from the Cargo workspace** because its `web-sys` / `leptos` dependencies only build for `wasm32`, which would break `cargo clippy --workspace` / `cargo test --workspace` on the host. Validate it separately:

```bash
cd src
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown --all-targets   # -D warnings
cargo test  --target wasm32-unknown-unknown                  # wasm-bindgen-test-runner + Node
trunk build                                                # emits ../dist
```

---

## License

This project is open source and available under the [MIT License](LICENSE).
