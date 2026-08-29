# PDF Splitter

```
██████╗ ██████╗ ███████╗ ██████╗██████╗ ██╗     ██╗████████╗████████╗███████╗██████╗
██╔══██╗██╔══██╗██╔════╝██╔════╝██╔══██╗██║     ██║╚══██╔══╝╚══██╔══╝██╔════╝██╔══██╗
██████╔╝██║  ██║█████╗  ███████╗██████╔╝██║     ██║   ██║      ██║   █████╗  ██████╔╝
██╔═══╝ ██║  ██║██╔══╝  ╚════██║██╔═══╝ ██║     ██║   ██║      ██║   ██╔══╝  ██╔══██╗
██║     ██████╔╝██║     ██████╔╝██║     ███████╗██║   ██║      ██║   ███████╗██║  ██║
╚═╝╚═════╝ ╚═╝╚═════╝╚═╝╚══════╝╚═╝   ╚═╝   ╚═╝╚══════╝╚═╝  ╚═╝
```

---

## ◆ PULSE

A multi-page PDF is a stack of documents wearing one cover. PDF Splitter
takes it apart - every page extracted into its own PDF file, fonts,
images, and embedded resources intact, processed in parallel across
every CPU core. A lean Tauri binary with a WASM Leptos UI: drag the
file onto the window, and the stack becomes pages. No Electron, no
JavaScript, no fidelity left on the floor.

| Parallel ▣ | 100% fidelity ▣ | Drag & drop ▣ | Native ▣ |
|---|---|---|---|

*The split loop - drop, process, save - is sealed.*

> Built with Tauri 2 + Leptos 0.7, split by `lopdf`, parallelized by
> `rayon` - a Rust engine that never touches a browser.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

One target, one tool, one command.

```
⟫ rustup target add wasm32-unknown-unknown
⟫ cargo install trunk
⟫ cargo tauri dev
```

UI hot-reloads at [http://127.0.0.1:1420](http://127.0.0.1:1420) while
the native window runs. Frontend only? `⟫ cd src && trunk serve`.

The release artifact: `⟫ cargo tauri build` - `.app`/`.dmg` on macOS,
`.msi`/`.exe` on Windows, in `src-tauri/target/release/bundle/`.

<details>
<summary>Prerequisites</summary>

- [Rust + Cargo](https://rustup.rs/) (>= 1.80) with the
  `wasm32-unknown-unknown` target
- [Trunk](https://trunk-rs.dev/) - installed above
- macOS: Xcode Command Line Tools; Windows: VS C++ build tools
- Optional (frontend unit tests only): Node.js + `wasm-bindgen-cli`

</details>

---

## ◆ ANATOMY

Three layers, one engine, zero wasted electrons.

- **Splits** - `crates/pdf-split-core` is the pure-Rust PDF engine with
  no Tauri dependency: page extraction via `lopdf`, fidelity guaranteed
  for fonts, images, and embedded resources.
- **Parallelizes** - `rayon` spreads the pages across every available
  core - a 300-page document becomes 300 parallel jobs, not a
  patience test.
- **Carries** - `src-tauri` is a thin IPC layer: commands in, files
  out, nothing between the engine and the filesystem.
- **Receives** - the Leptos frontend asks for nothing except the file:
  drag and drop onto the window, progress on screen, pages on disk.
- **Wears** - glassmorphism, dark-mode support, and smooth animations -
  a native-feel surface with a native engine underneath. The design
  language lives in `DESIGN.md`; the Vue-to-Leptos path in
  `MIGRATION.md`.

---

## ◆ RITUALS

**The core ceremony** - the stack becomes pages:

1. Drop the PDF onto the window. The app recognizes the offer without
   a single click-through dialog.
2. Watch the split: every core takes its pages; the progress reflects
   the parallelism.
3. Collect the pages - one PDF per page, every font and image where it
   belongs.
4. Send them on their way: one page to one inbox, one page to one
   folder, one page to one answer.

**The ceremony of fidelity** - nothing is re-rendered and nothing is
re-encoded through a browser pipeline. The page that comes out carries
the resources the page went in with - 100 percent, or the split is a
lie.

**The ceremony of the engine** - the core is pure Rust and testable
without a window: the split logic lives where the platform cannot
interfere, and the shell stays thin enough to ignore.

---

## ◆ ECHOES

**Where this artifact is heading**

```
split     ▸ lopdf page extraction, core crate ──────────────────────── ▸ sealed
parallel  ▸ rayon across all cores ─────────────────────────────────── ▸ sealed
deliver   ▸ Tauri bundle: .app/.dmg, .msi/.exe ─────────────────────── ▸ sealed
receive   ▸ drag & drop, WASM UI on 1420 ───────────────────────────── ▸ sealed
```

**Raising the artifact** - the conventions live in `AGENTS.md`; the
design language in `DESIGN.md`; the migration rationale in
`MIGRATION.md`. Gates: `cargo fmt --check`, clippy with `-D warnings`
on host and wasm targets, wasm tests via `wasm-bindgen-test`, and the
Trunk build. Open an issue first to discuss a change.

**Status** - Windows installers build from `v*` tags through the
[release workflow](.github/workflows/build-windows.yml).

---

```
  ─────────────────────────────────────────
   A stack of pages under one cover
   is still a stack of pages.
  ─────────────────────────────────────────
```

Open source under the [MIT License](LICENSE).