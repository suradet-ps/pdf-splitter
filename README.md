# PDF Splitter

> A fast, beautiful cross-platform desktop app that splits any multi-page PDF into individual page files — built with Tauri 2 and Vue 3.

![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8D8.svg)
![Vue](https://img.shields.io/badge/Vue-3.x-42b883.svg)
![TypeScript](https://img.shields.io/badge/TypeScript-5.x-3178C6.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey.svg)

---

## Overview

**PDF Splitter** is a cross-platform (macOS and Windows) desktop application that takes any multi-page PDF document and extracts every page into its own individual PDF file.

The app is built on a native [Tauri 2](https://tauri.app/) shell with a [Vue 3](https://vuejs.org/) + TypeScript renderer, providing:

- **Native performance** — a lean Rust binary with zero Electron overhead.
- **Parallel page processing** — automatically scales to all available CPU cores for fast extraction.
- **Beautiful, native-feel UI** — glassmorphism design, dark-mode support, and smooth animations.
- **Drag & drop** — drop a PDF straight onto the window to quickly begin splitting.
- **100% Fidelity** — guarantees perfect quality for fonts, images, and embedded resources.

---

## Getting Started

### Prerequisites

- [Rust + Cargo](https://rustup.rs/) (≥ 1.80)
- [Node.js](https://nodejs.org/) (≥ 18)
- **macOS**: Xcode Command Line Tools (`xcode-select --install`)
- **Windows**: Build Tools for Visual Studio (C++ build tools)

### Development

```bash
# 1. Clone the repository
git clone https://github.com/suradet-ps/pdf-splitter.git
cd pdf-splitter

# 2. Install frontend dependencies
npm install

# 3. Start the development server (hot-reload for UI + Rust)
npm run tauri dev
```

### Production Build

#### macOS

```bash
# Build the optimized binary + macOS app bundle + DMG installer
npm run tauri build
```

The output artifacts (macOS `.app` bundle and `.dmg` installer) will be generated in the `src-tauri/target/release/bundle/` directory.

#### Windows

You can build the app locally on Windows using the same command:

```bash
npm run tauri build
```

The output artifacts (`.msi` or `.exe` installer) will be generated in the `src-tauri/target/release/bundle/` directory.

*Note: There is also an automated GitHub Actions workflow (`build-windows.yml`) that automatically builds and releases the Windows installer (.msi/.exe) whenever a new `v*` tag is pushed to the repository.*

---

## Tech Stack

- **Backend**: Rust, Tauri 2, `lopdf` (for PDF processing), `rayon` (for parallel processing).
- **Frontend**: Vue 3 (Composition API), TypeScript, Vite.

---

## License

This project is open source and available under the [MIT License](LICENSE).
