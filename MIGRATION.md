# Frontend Migration: Vue 3 → Leptos (CSR)

This document records the migration of the PDF Splitter desktop app's frontend
from **Vue 3 + TypeScript + Vite** to **Leptos 0.7 (client-side rendered) in
Rust**, built with **Trunk**.

It is both a record of what was done and a reference for how the frontend is
structured. Only the frontend framework changed — functionality, appearance,
CSS, UX, and the overall architecture were preserved. All business logic
continues to live in the pure-Rust `pdf-split-core` crate and the thin
`src-tauri` command layer; the frontend is UI-only.

---

## Status

**Complete.** The Vue renderer has been fully replaced. All quality gates pass:

| Gate | Result |
|------|--------|
| `cargo build --target wasm32-unknown-unknown` | ✅ |
| `cargo clippy --target wasm32-unknown-unknown --all-targets` | ✅ 0 warnings |
| `cargo fmt --check` | ✅ |
| `cargo test --target wasm32-unknown-unknown` | ✅ 6 tests |
| `trunk build` | ✅ |
| Workspace `cargo fmt` / `clippy -D warnings` / `test` | ✅ 52 tests |
| No JavaScript / TypeScript / npm / bun / Vite remains | ✅ |

---

## Goals & Principles

The migration was a **port of behavior, not a translation of syntax**. For each
Vue component we understood what it did and rebuilt the same behavior with
idiomatic Leptos, rather than translating line by line.

Guiding rules that were followed:

- **Rust only.** No JavaScript, TypeScript, npm, bun, webpack, or Vite.
- **Business logic stays in Rust** (`pdf-split-core` + `src-tauri`). Leptos is
  responsible for UI only.
- **UI never calls `invoke` directly** — it goes through a service layer.
- **No `unwrap` / `expect` / `panic!` in non-test code**; fallible operations
  return `Result`.
- **CSS preserved verbatim** — no Tailwind, no CSS framework, no rewrite. All
  variables, animations, spacing, typography, colors, class names, and the root
  `data-state` hook are unchanged, so appearance and UX match the original.
- **Signals own state, they don't replace it** — prefer plain Rust structs and
  minimal signals; avoid `Rc<RefCell<_>>` unless genuinely required.

---

## Technology Stack

| Concern | Before (Vue) | After (Leptos) |
|---------|--------------|----------------|
| UI framework | Vue 3 (SFC) | Leptos 0.7 (`features = ["csr"]`) |
| Language | TypeScript | Rust → `wasm32-unknown-unknown` |
| Build tool | Vite | Trunk 0.21 |
| Package manager | bun / npm | Cargo |
| Lint / format | Biome | Clippy + rustfmt |
| State management | Pinia / composable (`ref`) | `RwSignal` + `provide_context` |
| Tauri IPC | `@tauri-apps/api` (npm) | `window.__TAURI__` global via `web-sys` |
| Unit tests | vitest (none for FE) | `wasm-bindgen-test` on the wasm target |

Crate versions are declared **explicitly** in `src/Cargo.toml` (no
`workspace = true` inheritance, matching the repo's convention that a crate can
be lifted out and built in isolation):

- `leptos = { version = "0.7", features = ["csr"] }`
- `wasm-bindgen = "0.2"`, `wasm-bindgen-futures = "0.4"`, `js-sys = "0.3"`
- `web-sys = "0.3"` — features: `Window`, `Event`, `MouseEvent`,
  `KeyboardEvent`, `DragEvent`, `DataTransfer`, `File`, `FileList`,
  `HtmlElement`, `HtmlInputElement`
- `serde = "1"`, `serde-wasm-bindgen = "0.6"`, `serde_json = "1"`
- `console_error_panic_hook = "0.1"`
- dev-dependency: `wasm-bindgen-test = "0.3"`

---

## Project Layout

The frontend crate lives in `src/` and is **excluded from the Cargo
workspace**. Its `web-sys` / `leptos` dependencies only build for `wasm32`, so
including it would break `cargo clippy --workspace` / `cargo test --workspace`
on the host. It is built and validated separately with `trunk`.

```
src/
├── Cargo.toml            # frontend crate manifest (cdylib, explicit deps)
├── Trunk.toml            # dist = ../dist, public_url = /, dev server :1420
├── index.html            # trunk entry: links CSS + rust
├── .cargo/config.toml    # registers the wasm test runner
├── assets/styles/        # CSS ported verbatim from the Vue app
│   ├── tokens.css        # design tokens (CSS variables)
│   ├── base.css          # element resets / base styles
│   ├── global.css        # component classes, utilities, animations
│   └── main.css          # aggregator (@import of the three above)
└── src/
    ├── lib.rs            # crate root, module tree, #[wasm_bindgen(start)]
    ├── models.rs         # data models + pure helpers (unit-tested)
    ├── pages/
    │   ├── mod.rs
    │   └── app.rs        # App root component
    ├── components/
    │   ├── mod.rs
    │   ├── drop_zone.rs
    │   ├── file_card.rs
    │   ├── progress_view.rs
    │   ├── result_view.rs
    │   └── error_view.rs
    ├── contexts/
    │   ├── mod.rs
    │   └── pdf_splitter.rs
    └── services/
        ├── mod.rs
        ├── tauri.rs      # low-level window.__TAURI__ bindings
        └── commands.rs   # typed per-command wrappers
```

`src-tauri/tauri.conf.json` was switched to the Trunk toolchain:
`beforeDevCommand: "trunk serve"`, `beforeBuildCommand: "trunk build"`,
`devUrl: http://127.0.0.1:1420`, `frontendDist: "../dist"`, and
`withGlobalTauri: true` (required so the wasm frontend can reach
`window.__TAURI__` without an npm import).

---

## File Mapping (Vue → Rust)

Each Vue Single File Component became one Rust module.

| Vue file (removed) | Rust module (added) | Role |
|--------------------|---------------------|------|
| `src/App.vue` | `src/src/pages/app.rs` | Root view, state-driven content switch, action wiring |
| `src/components/DropZone.vue` | `src/src/components/drop_zone.rs` | Idle-state drop area + file picker |
| `src/components/FileCard.vue` | `src/src/components/file_card.rs` | Ready-state file info + split action |
| `src/components/ProgressView.vue` | `src/src/components/progress_view.rs` | Processing-state progress bar / dots / stream |
| `src/components/ResultView.vue` | `src/src/components/result_view.rs` | Complete-state file list + reveal |
| `src/components/ErrorView.vue` | `src/src/components/error_view.rs` | Error-state message + hint + retry |
| `src/composables/usePdfSplitter.ts` | `src/src/contexts/pdf_splitter.rs` | Central state + async actions |
| `src/types/index.ts` | `src/src/models.rs` | Data models + pure formatting helpers |
| `src/main.ts` | `src/src/lib.rs` | Entry point / mount |
| inline Tauri `invoke` calls | `src/src/services/{tauri,commands}.rs` | IPC glue, isolated from UI |

### Removed toolchain files

No JavaScript, TypeScript, or Node tooling remains:

```
biome.json            package.json          bun.lock
index.html (old)      vite.config.ts
tsconfig.json         tsconfig.node.json
src/App.vue           src/main.ts           src/vite-env.d.ts
src/types/index.ts    src/composables/usePdfSplitter.ts
src/components/*.vue   (DropZone, FileCard, ProgressView, ResultView, ErrorView)
```

---

## Architecture

```
Leptos components (pages/, components/)
        │  props + Callback
        ▼
PdfSplitterContext  (contexts/pdf_splitter.rs)   ← provide_context / use_context
        │  RwSignal state + async actions (spawn_local)
        ▼
services/commands.rs   (typed wrappers → Result<T, PdfError>)
        │
        ▼
services/tauri.rs      (window.__TAURI__.core.invoke / event.listen via web-sys)
        │
        ▼
src-tauri commands  →  pdf-split-core (pure Rust engine)
```

- **Components are presentational.** They own only local UI state (drag hover,
  hovered row). All application state and async actions arrive via props /
  `Callback`s from `App`.
- **`App` wires actions.** It reads the context, builds `Callback`s that
  `spawn_local` into context actions, and swaps the content region based on
  state.
- **`PdfSplitterContext`** is the Leptos equivalent of the old Pinia store /
  composable: a `Copy` struct of `RwSignal` fields plus derived `Memo`s and the
  async transition methods. It is passed by value into children.
- **Services** are the only code that touches Tauri IPC.

### Reactivity mapping

| Vue | Leptos |
|-----|--------|
| `ref()` | `RwSignal` |
| `computed()` | `Memo` |
| `watch()` | `Effect` |
| `provide()` / `inject()` | `provide_context()` / `use_context()` |
| `v-if` | `<Show>` / `move || cond.then(…)` |
| `v-for` | `<For>` |
| `@click` | `on:click` |
| `v-model` | signal + `prop:value` + `on:input` |

### State model

Five discrete states drive the single content region and the root `data-state`
attribute (preserving the original CSS hooks):

| `AppState` | Component shown | `data-state` |
|------------|-----------------|--------------|
| `Idle` | `DropZone` | `idle` |
| `Ready` | `FileCard` | `ready` |
| `Processing` | `ProgressView` | `processing` |
| `Complete` | `ResultView` | `complete` |
| `Error` | `ErrorView` | `error` |

```
Idle ──pick_file──▶ Ready ──start_split──▶ Processing ──▶ Complete
  ▲                   │                         │            │
  └──────── reset ────┴──────── error ──────────┴────────────┘
                                (any failure) ▶ Error
```

---

## Tauri IPC Contract

The frontend re-declares the wire types in `models.rs` rather than importing
the core crate — this keeps the wasm bundle lean, and the JSON shape (not the
Rust types) is the real contract. Every field and name was verified against
`src-tauri/src/commands.rs` and `crates/pdf-split-core`.

| Command / event | JS args (camelCase) | Returns / payload |
|-----------------|---------------------|-------------------|
| `pick_pdf_file` | — | `Option<String>` (path; `None` = cancelled) |
| `get_file_info` | `path` | `{ pageCount, sizeBytes }` |
| `pick_output_dir` | — | `Option<String>` |
| `split_pdf` | `inputPath`, `outputDir` | `{ totalPages, outputFiles, elapsedMs }` |
| `reveal_in_finder` | `path` | `()` |
| `split://progress` | (event) | `{ current, total, fileName }` |

Errors from `pdf_split_core::PdfError` serialize as `{ "kind", "message" }`,
where `kind ∈ { FileNotFound, InvalidPdf, Io, NoPages, Internal }`.
`models::PdfError::from_raw` decodes this into a `PdfErrorKind` enum + message;
`ErrorView` maps each kind to a title and a user-facing hint.

Tauri v2 bridges camelCase JS keys to snake_case Rust parameters, so the
frontend sends `inputPath` / `outputDir` to match the backend's `input_path` /
`output_dir`.

---

## Notable Implementation Patterns

1. **Global Tauri, not npm import.** Trunk has no JS bundler, so
   `@tauri-apps/api` cannot be `import`ed. `withGlobalTauri: true` exposes
   `window.__TAURI__`; `services/tauri.rs` reaches `.core.invoke` and
   `.event.listen` through `js_sys::Reflect` + `web-sys`.

2. **Progress throttling via `requestAnimationFrame`.** High-frequency
   `split://progress` events are buffered in an `Rc<RefCell<Option<..>>>` and
   flushed at most once per animation frame into the `operation` signal — this
   avoids repainting faster than the display refresh, mirroring the original.
   This is the single sanctioned `Rc<RefCell<_>>`: the event callback and the
   rAF closure are both `'static` and must share mutable state outside the
   reactive system.

3. **`StoredValue` for per-row owned data.** Inside `<For>` / `<Show>` bodies,
   `String`s that must survive re-renders (file name, path) are wrapped in
   `StoredValue` so the child closures stay `Fn` (not `FnOnce`) and remain
   `Copy`.

4. **Type-erased branches with `AnyView`.** The five-way state `match` in `App`
   unifies its arms via `.into_any()` so the closure returns one concrete type.

5. **Models re-declared, not re-exported.** `models.rs` mirrors the backend
   JSON contract with matching serde conventions (`rename_all = "camelCase"`),
   keeping the wasm crate independent of the host-only core crate.

---

## Fixes Applied During the Port

The initial port compiled with many errors; these were resolved to reach a
green build under the workspace's strict `-D warnings` policy.

**Compilation**

- Corrected crate-relative paths: bare `models::` / `services::` →
  `crate::models::` / `crate::services::`.
- `use leptos::spawn_local` → `use leptos::task::spawn_local` (correct path in
  Leptos 0.7).
- Removed invalid trailing `;` inside `view!` `on:click=…` attribute closures
  (6 sites).
- Fixed *borrow of moved value* on `String` props (`file_card.rs` `file_name`,
  `result_view.rs` `name` / `path`) by switching to `StoredValue`.
- Braced `Show when=move || { dot_count.get() >= 2 }` so the macro parsed a
  `bool` instead of a `u32`.

**Clippy (`clippy::all`, `-D warnings`)**

- Removed `.clone()` on `Copy` `Callback`s (`clone_on_copy`).
- Simplified `!(a && !b)` → `!a || b` (`nonminimal_bool`).
- `t <= 0` (u32) → `t == 0` (`absurd_extreme_comparisons`).
- Removed dead code: unused context methods (`progress_label`,
  `output_file_names`), the unread `SplitOperation.output_dir` field, and
  unused imports.

**Bug caught by the unit tests**

- `models::default_output_dir` stripped only lowercase `.pdf`. Made the
  extension check case-insensitive so `report.PDF` → `./report`.

---

## Runtime Bugs Fixed After the Port

These surfaced only at runtime (in the browser / Tauri webview), not at
compile time, and are worth recording because the root causes are easy to
repeat in other Leptos + Tauri projects.

### 1. Tauri global API fails to load from a CDN in a no-bundler setup

The frontend is built by Trunk, which has **no JS bundler**, so the
`@tauri-apps/api` npm package cannot be `import`ed normally. The original
approach relied on `withGlobalTauri: true` to inject `window.__TAURI__`
natively — but **`withGlobalTauri` is rejected by some Tauri v2 configs**, so
the global must instead be loaded from a CDN at runtime.

The naive loader did:

```js
const code = await (await fetch("https://cdn.jsdelivr.net/npm/@tauri-apps/api@2/+esm")).text();
const blobUrl = URL.createObjectURL(new Blob([code], { type: "text/javascript" }));
const mod = await import(blobUrl);   // ❌ fails
```

This throws `Module name, '/@tauri-apps/api@2.11.1/es2022/api.mjs' does not
resolve to a valid URL`. **Why:** both jsdelivr `+esm` and esm.sh rewrite the
package's internal imports to *absolute* paths
(`/@tauri-apps/api@2.11.1/es2022/api.mjs`). Those resolve fine against the CDN's
own origin, but a `blob:` URL has an **opaque origin**, so the absolute path
cannot be resolved → load failure → `window.__TAURI__` stays `undefined` →
"Tauri global API is not available" on the first IPC call.

**Fix:** import the package *directly* from esm.sh (no `fetch` + `blob`
round-trip). The absolute sub-imports then resolve against esm.sh's own origin:

```js
window.__TAURI_PROMISE__ = import("https://esm.sh/@tauri-apps/api@2")
  .then((mod) => { window.__TAURI__ = mod; return mod; })
  .catch((err) => { console.error("Failed to load Tauri global API:", err); throw err; });
```

And widen the Tauri CSP so the cross-origin module is permitted:

```jsonc
"csp": "…; script-src 'self' 'unsafe-inline' https://esm.sh; …"
```

**Takeaway for other projects:** when loading an npm ESM package into a
no-bundler WASM frontend, import it directly from a CDN that keeps full URLs
(esm.sh), never via a `blob:` URL. If `withGlobalTauri` is available, prefer
the native global and skip the CDN entirely.

### 2. `already been disposed` panics when a state-driven view unmounts

The root `App` component renders a different child per `AppState` via a
re-running closure:

```rust
let content = move || -> AnyView {
    match ctx.state.get() {
        AppState::Idle => view! { <DropZone busy=ctx.is_busy.into() …/> }.into_any(),
        // …
    }
};
```

`ctx.is_busy.into()` creates a `Signal`. In Leptos, `Signal::from(RwSignal)`
**registers a new `ArenaItem` owned by the current reactive owner** — here, the
per-render effect that runs `content`. So every `state` change disposed the
previous render's `busy` signal. When `DropZone` (still subscribed to `busy`)
was torn down a frame later, a final read hit the disposed signal and panicked
with `you tried to access a reactive value … but it has already been disposed`
(attributed to the signal's creation site in `app.rs`, not the read site).

**Fix:** build every `Signal` prop **once** at the `App` scope (outside the
re-running closure), so its `ArenaItem` is owned by `App` and lives for the whole
app lifetime:

```rust
let busy: Signal<bool> = ctx.is_busy.into();   // created once, at App scope
// …then inside content:  <DropZone busy=busy …/>
```

**Takeaway for other projects:** never create a `Signal` / `Memo` / `ReadSignal`
*inside* a closure that re-runs on reactive updates (a `view!` branch, a
`Show` `when`/`fallback`, a `content` switch, a `For` `each`). Hoist derived
values and signal-wrappers to a stable owner, or read with `.get()` (which does
not create an `ArenaItem`) when you only need a snapshot value.

### 3. `closure invoked recursively or after being dropped`

`requestAnimationFrame` was scheduled with `Closure::once` + `forget()`. If the
browser ever invokes that closure a second time (or after teardown),
`Closure::once` panics with `closure invoked recursively or after being dropped`.

**Fix:** use a repeatable `Closure::wrap` guarded by an `Option` so the callback
runs at most once but a stray second call is a safe no-op:

```rust
let slot = Rc::new(RefCell::new(Some(Box::new(cb) as Box<dyn FnOnce()>)));
let closure_slot = slot.clone();
let closure = Closure::wrap(Box::new(move |_ts: JsValue| {
    if let Some(cb) = closure_slot.borrow_mut().take() { cb(); }
}) as Box<dyn FnMut(JsValue)>);
let _ = win.request_animation_frame(closure.as_ref().unchecked_ref());
closure.forget();
```

**Takeaway for other projects:** for fire-once JS callbacks, prefer
`Closure::wrap` + an `Option`/`Cell` one-shot guard over `Closure::once` when
the registration can outlive the logical event or be invoked more than once.

### 4. Reactive-tracking warning in async actions

Reading a signal inside an `async fn` (`pick_file`, `start_split`, …) — e.g.
`if self.is_busy.get()` — fires `you access a reactive value … outside a
reactive tracking context` because no `Observer` is active during a `Future`.
These reads are intentionally non-tracking, so use `.get_untracked()` instead of
`.get()` in async code. This silences the warning and documents intent.

---

## Testing

Pure-logic helpers in `models.rs` (`shorten_dir`, `default_output_dir`,
`format_bytes`, `format_duration`, `basename`, `PdfError::from_raw`) are unit
tested. Because the crate only builds for `wasm32`, the tests are annotated with
`#[wasm_bindgen_test]` and executed by `wasm-bindgen-test-runner` (Node
backend), registered in `src/.cargo/config.toml`:

```toml
[target.wasm32-unknown-unknown]
runner = "wasm-bindgen-test-runner"
```

Two `cfg(test)` accommodations keep the test build clean:

- The `#[wasm_bindgen(start)]` entry point is disabled under `cfg(test)` (it
  otherwise collides with the test harness's entry symbol).
- `#![cfg_attr(test, allow(dead_code, unused_imports))]` silences noise from the
  UI module tree, which the pure-logic tests don't exercise.

One-time prerequisite: `cargo install wasm-bindgen-cli --version 0.2.126`
(provides `wasm-bindgen-test-runner`). Node is required to run the tests.

---

## Verification

The frontend crate is validated separately from the workspace.

```bash
# ── Frontend (wasm32) ──────────────────────────────────────────────
cd src
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown --all-targets   # -D warnings via .cargo/config
cargo test  --target wasm32-unknown-unknown                  # needs wasm-bindgen-test-runner + Node
trunk build                                                  # emits ../dist

# ── Workspace (host: core + tauri) ─────────────────────────────────
cd ..
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# ── Full app bundle ────────────────────────────────────────────────
cargo tauri build      # runs `trunk build` via beforeBuildCommand
```

> The legacy `bun run type-check` / `bun run check` steps no longer apply — the
> Node/Biome toolchain was removed with the Vue frontend and is superseded by
> the Clippy/rustfmt/Trunk commands above.

---

## Known Deviations & Follow-ups

- **Mount target.** The app mounts via `mount_to_body`; `index.html` still
  contains an unused empty `<div id="app">`. No CSS targets `#app`, so
  appearance is unaffected; the div can be removed in a later cleanup.
- **`assets/styles/main.css`** is an `@import` aggregator kept for parity;
  `index.html` links the three underlying stylesheets directly, so `main.css`
  is currently unreferenced (harmless).
- **`AGENTS.md`** still references the removed `bun run` verification steps and
  should be updated to the Rust/Trunk commands listed above.
