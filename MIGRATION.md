# Frontend Migration Playbook: Vue 3 + Vite → Leptos 0.7 (CSR) + Trunk

A **reusable playbook** for migrating an SPA frontend to **Leptos 0.7, client-side
rendered, compiled to `wasm32-unknown-unknown` via Trunk**. It was distilled from
a real migration of this repo's frontend (Vue 3 + TypeScript + Vite → Leptos +
Trunk, inside a Tauri desktop app). The concrete file/command examples come from
that migration, but the *procedure* and the *gotchas* generalize to any
SPA → Leptos CSR port.

**How to use this as an AI agent:** read §2 top-to-bottom as the execution
checklist, consult §3 before touching IPC, and use §4 as a debugging lookup
(when the build runs but the app misbehaves). The reference tables in §6 are for
accuracy, not procedure.

---

## 0. When to use this playbook

Use it when the goal is: **replace the UI framework, keep everything else**. The
migration here changed only the frontend; business logic stayed in a pure-Rust
crate and a thin Tauri command layer. If you must also re-architect the backend,
this playbook covers UI only.

**Red lines (non-negotiable for this style of port):**

- **Rust only in the frontend.** No JavaScript, TypeScript, npm, bun, webpack,
  or Vite. Leptos is UI-only.
- **Business logic stays out of the UI.** The frontend never calls the backend
  directly; it goes through a typed service layer.
- **No `unwrap` / `expect` / `panic!` in non-test code** — fallible ops return
  `Result`.
- **Preserve CSS verbatim.** No Tailwind, no CSS framework, no rewrite. Keep all
  class names, design tokens, animations, and any root `data-*` state hooks so
  appearance/UX match the original.

---

## 1. Target architecture & toolchain

| Concern | After (Leptos) |
|---------|----------------|
| UI framework | Leptos 0.7 (`features = ["csr"]`) |
| Language | Rust → `wasm32-unknown-unknown` |
| Build tool | Trunk |
| Lint / format | Clippy (`-D warnings`) + rustfmt |
| State | `RwSignal` + `provide_context` (the Pinia/composable equivalent) |
| IPC (Tauri) | `window.__TAURI__` global via `web-sys` (NOT an npm import) |
| Unit tests | `wasm-bindgen-test` on the wasm target |

**Crate isolation rule.** The wasm frontend crate must be **excluded from the
Cargo workspace** (its `web-sys`/`leptos` deps only build for `wasm32`, so
including it breaks `cargo clippy --workspace` / `cargo test --workspace` on the
host). Declare **all dependency versions explicitly** in its `Cargo.toml` (no
`workspace = true` inheritance) so the crate can be lifted out and built alone.

Typical `Cargo.toml` deps: `leptos = { version = "0.7", features = ["csr"] }`,
`wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys` (enable the exact
features you use: `Window`, `DragEvent`, `DataTransfer`, `File`, `FileList`,
`HtmlElement`, `HtmlInputElement`, …), `serde`, `serde-wasm-bindgen`,
`console_error_panic_hook`; dev: `wasm-bindgen-test`.

`Trunk.toml`: `dist = "../dist"`, `public_url = "/"`, dev server on a fixed port
(`:1420` here). `index.html` links the CSS and the `rust` entry; the wasm mounts
via `#[wasm_bindgen(start)]` + `mount_to_body`.

---

## 2. Migration procedure

Port **behavior, not syntax**: for each component, understand what it does and
rebuild it idiomatically in Leptos. Work bottom-up so dependents always compile.

### Step 1 — Scaffold the crate
- Create the crate outside the workspace; add explicit deps; add
  `.cargo/config.toml` with the wasm test runner
  (`[target.wasm32-unknown-unknown] runner = "wasm-bindgen-test-runner"`).
- Create `index.html` (CSS links + `<link rel="rust">`) and `Trunk.toml`.
- **CHECKPOINT:** `trunk build` produces `../dist` with an empty mount point.

### Step 2 — Port CSS verbatim
- Copy the stylesheets as-is into `assets/styles/`. Do **not** rename classes or
  restructure selectors. Keep the root `data-state` (or equivalent) hook that
  CSS uses to swap themes — you will reproduce those state values exactly in
  Leptos.
- **DON'T** reach for a CSS framework. **DO** keep the original class names so
  the ported `view!` markup lines up with the existing styles.

### Step 3 — Port models / types
- Re-declare the backend **wire types** in a `models.rs` (mirror JSON shape with
  `serde(rename_all = "camelCase")`). **DON'T** import the backend crate — the
  wasm crate must stay independent; the JSON contract is the real interface.
- Put pure formatting helpers (`format_bytes`, `shorten_dir`, `basename`, …)
  here and **unit-test them** (see §5).

### Step 4 — Port state management
- Map Pinia store / composable → a `Copy` struct holding `RwSignal` fields +
  derived `Memo`s + async transition methods, registered with
  `provide_context`. Pass it by value into children.
- **WATCH OUT (signal lifetime):** create every `Signal`/`Memo` you pass as a
  prop **once at the owning component's scope**, never inside a closure that
  re-runs on reactive updates. See §4 #2.

### Step 5 — Port components
- One Rust module per Vue SFC. Keep components **presentational**: local UI
  state only (drag hover, hovered row), everything else via props + `Callback`.
- Reactivity map:

  | Vue | Leptos |
  |-----|--------|
  | `ref()` | `RwSignal` |
  | `computed()` | `Memo` |
  | `watch()` | `Effect` |
  | `provide`/`inject` | `provide_context`/`use_context` |
  | `v-if` | `<Show>` / `move || cond` |
  | `v-for` | `<For>` |
  | `@click` | `on:click` |
  | `v-model` | signal + `prop:value` + `on:input` |

- **DO** drive a multi-state UI with a `match` over a state `RwSignal`, unifying
  arms via `.into_any()` so the switch closure returns one concrete type.
- **WATCH OUT:** inside `<For>` / `<Show>` bodies, wrap owned `String`s (file
  name, path) in `StoredValue` so child closures stay `Fn`/`Copy` (avoids
  *borrow of moved value*).
- **DON'T** put a trailing `;` inside a `view!` `on:click=move |_| …` closure.
- **CHECKPOINT:** `cargo clippy --target wasm32-unknown-unknown --all-targets`
  is clean (this catches most port errors — see §4 port-time fixes).

### Step 6 — Wire backend IPC (read §3 first)
- `services/tauri.rs`: reach `window.__TAURI__.core.invoke` / `.event.listen`
  via `js_sys::Reflect` + `web-sys`. Await a load promise so the first call
  can't race the global's availability.
- `services/commands.rs`: one typed wrapper per command returning `Result<T, E>`.
- **DON'T** call `invoke` from UI code.

### Step 7 — Progress / high-frequency events
- Buffer events in `Rc<RefCell<Option<..>>>` and flush at most once per
  `requestAnimationFrame` into a signal (avoids repainting faster than the
  display). This is the one sanctioned `Rc<RefCell<_>>` — the callback and the
  rAF closure are both `'static`. **WATCH OUT:** schedule rAF with a repeatable
  closure, not `Closure::once` (see §4 #3).

### Step 8 — Tests & verification
- Unit-test pure helpers. Run the full gate in §5.
- **CHECKPOINT:** `trunk build` succeeds and the app mounts.

---

## 3. Tauri IPC in a no-bundler frontend (CRITICAL)

Trunk has **no JS bundler**, so `@tauri-apps/api` cannot be `import`ed the normal
way. There are two ways to expose `window.__TAURI__`:

1. **Preferred:** `withGlobalTauri: true` in `tauri.conf.json` injects the global
   natively. **WATCH OUT:** this field is **rejected by some Tauri v2 configs**.
   If the config errors out, you cannot rely on it.
2. **Fallback:** load the package from a CDN at runtime.

**If you load from a CDN, you MUST import it directly — never via a `blob:`
URL.** Both jsdelivr `+esm` and esm.sh rewrite the package's *internal* imports
to **absolute** paths (`/@tauri-apps/api@2.11.1/es2022/api.mjs`). Those resolve
against the CDN's own origin, but a `blob:` URL has an **opaque origin**, so the
absolute path cannot resolve →

```
Module name, '/@tauri-apps/api@2.11.1/es2022/api.mjs' does not resolve to a valid URL
```

✅ **DO** (direct import — sub-imports resolve against esm.sh's origin):

```js
window.__TAURI_PROMISE__ = import("https://esm.sh/@tauri-apps/api@2")
  .then((mod) => { window.__TAURI__ = mod; return mod; })
  .catch((err) => { console.error("Failed to load Tauri global API:", err); throw err; });
```

❌ **DON'T** (fetch source → `blob:` → `import(blobUrl)` — fails as above).

And widen the Tauri CSP so the cross-origin module is allowed:

```jsonc
"csp": "…; script-src 'self' 'unsafe-inline' https://esm.sh; …"
```

**Rule of thumb:** in a no-bundler WASM frontend, import npm ESM packages
*directly* from a CDN that keeps full URLs (esm.sh). If `withGlobalTauri` is
available, prefer the native global and skip the CDN entirely.

---

## 4. Gotchas & debugging playbook

Compile errors are usually caught by `cargo clippy` (run it early and often
under `-D warnings`). The dangerous bugs are **runtime-only** — use this
lookup when the build is green but the app breaks.

### #1 — Tauri global API is not available / module does not resolve
- **Symptom:** console `Failed to load Tauri global API`, or
  `Module name, '/@tauri-apps/api@…/es2022/api.mjs' does not resolve to a valid
  URL`; first IPC call errors with `Tauri global API is not available`.
- **Cause:** loaded the npm package via `fetch` + `blob:` URL (absolute
  sub-imports can't resolve from an opaque `blob:` origin), or CSP blocks the
  CDN, or `withGlobalTauri` is disallowed and nothing loads the global.
- **Fix:** import directly from esm.sh (§3); add `https://esm.sh` to CSP
  `script-src`; if `withGlobalTauri` works in your config, use the native
  global instead.

### #2 — `you tried to access a reactive value … but it has already been disposed`
- **Symptom:** panic attributed to a signal's *creation* site (e.g. `app.rs`),
  fired while a child view (e.g. a drop zone) is unmounting.
- **Cause:** a `Signal` (or `Memo`) was created **inside** a closure that
  re-runs on reactive updates — a `view!` branch, a `Show` `when`/`fallback`, a
  state-switch `content` closure, a `For` `each`. `Signal::from(RwSignal)`
  registers a new `ArenaItem` **owned by the current reactive owner**, so each
  re-render disposes the previous signal; a late read during teardown panics.
- **Fix:** hoist every signal/memo you pass as a prop to a **stable owner** (the
  component that owns the state), created once. If you only need a snapshot
  value inside a re-running closure, read with `.get()` (does not create an
  `ArenaItem`) instead of creating a `Signal`.
- **Rule:** *never* create a `Signal`/`Memo`/`ReadSignal` inside a re-running
  reactive closure.

### #3 — `closure invoked recursively or after being dropped`
- **Symptom:** runtime panic from a `wasm-bindgen` closure trampoline, often on
  `requestAnimationFrame` / event callbacks.
- **Cause:** scheduled a fire-once JS callback with `Closure::once` + `forget()`.
  A stray second invocation (or one after teardown) panics `Closure::once`.
- **Fix:** use a repeatable `Closure::wrap` guarded by an `Option` so the
  callback runs at most once but extra calls are safe no-ops:

  ```rust
  let slot = std::rc::Rc::new(std::cell::RefCell::new(Some(Box::new(cb) as Box<dyn FnOnce()>)));
  let closure_slot = slot.clone();
  let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_ts: JsValue| {
      if let Some(cb) = closure_slot.borrow_mut().take() { cb(); }
  }) as Box<dyn FnMut(JsValue)>);
  let _ = win.request_animation_frame(closure.as_ref().unchecked_ref());
  closure.forget();
  ```
- **Rule:** for fire-once JS callbacks that may be invoked again or outlive the
  event, prefer `Closure::wrap` + `Option`/`Cell` guard over `Closure::once`.

### #4 — `you access a reactive value … outside a reactive tracking context`
- **Symptom:** console warning (not a panic) when reading a signal inside an
  `async fn`.
- **Cause:** no `Observer` is active during a `Future`, so `.get()` reads
  untracked and Leptos warns.
- **Fix:** use `.get_untracked()` for the non-tracking reads inside async
  actions (and `.set_untracked()` if writing outside tracking). This silences
  the warning and documents intent.

### Port-time `clippy` / compile fixes (apply as you hit them)
- Crate-relative paths: bare `models::`/`services::` → `crate::models::`/
  `crate::services::`.
- `use leptos::spawn_local` → `use leptos::task::spawn_local` (Leptos 0.7).
- *borrow of moved value* on `String` props → wrap in `StoredValue`.
- `Show when=move || { … }` must brace a `bool` expression so the macro parses
  a `bool`, not a `u32`.
- `.clone()` on `Copy` `Callback`s → drop the clone (`clone_on_copy`).
- `!(a && !b)` → `!a || b` (`nonminimal_bool`); `u32 <= 0` → `== 0`
  (`absurd_extreme_comparisons`).
- Remove dead context methods / unused fields / unused imports.
- **Bug caught by tests:** case-insensitive file-extension checks
  (`report.PDF` must still strip `.pdf`).

---

## 5. Verification gate

Run the frontend crate separately from the workspace.

```bash
# ── Frontend (wasm32) ──────────────────────────────────────────────
cd src
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown --all-targets   # -D warnings
cargo test  --target wasm32-unknown-unknown                  # wasm-bindgen-test-runner + Node
trunk build                                                  # emits ../dist

# ── Workspace (host: core + tauri) ─────────────────────────────────
cd ..
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# ── Full app bundle ────────────────────────────────────────────────
cargo tauri build      # runs `trunk build` via beforeBuildCommand
```

One-time prerequisite for frontend tests: `cargo install wasm-bindgen-cli
--version 0.2.126` (provides `wasm-bindgen-test-runner`); Node is required to
run the tests. Under `cfg(test)`, disable `#[wasm_bindgen(start)]` and add
`#![cfg_attr(test, allow(dead_code, unused_imports))]` so the unused UI module
tree doesn't fail the pure-logic test build.

> The legacy `bun run type-check` / `bun run check` steps no longer apply — the
> Node/Biome toolchain was removed with the Vue frontend.

---

## 6. Reference data (this repo's migration)

### File mapping (Vue → Rust)

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

Removed toolchain files: `biome.json`, `package.json`, `bun.lock`, old
`index.html`, `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`,
`src/App.vue`, `src/main.ts`, `src/vite-env.d.ts`, `src/types/index.ts`,
`src/composables/usePdfSplitter.ts`, `src/components/*.vue`.

### Tauri IPC contract

The frontend re-declares wire types in `models.rs` (JSON shape is the real
contract, verified against `src-tauri/src/commands.rs` and `crates/pdf-split-core`).

| Command / event | JS args (camelCase) | Returns / payload |
|-----------------|---------------------|-------------------|
| `pick_pdf_file` | — | `Option<String>` (path; `None` = cancelled) |
| `get_file_info` | `path` | `{ pageCount, sizeBytes }` |
| `pick_output_dir` | — | `Option<String>` |
| `split_pdf` | `inputPath`, `outputDir` | `{ totalPages, outputFiles, elapsedMs }` |
| `reveal_in_finder` | `path` | `()` |
| `split://progress` | (event) | `{ current, total, fileName }` |

Errors serialize as `{ "kind", "message" }` (`kind ∈ { FileNotFound,
InvalidPdf, Io, NoPages, Internal }`); `models::PdfError::from_raw` decodes it.
Tauri v2 bridges camelCase JS keys to snake_case Rust params, so the frontend
sends `inputPath` / `outputDir`.

### State model

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

### Project layout

```
src/                      # frontend crate (excluded from workspace)
├── Cargo.toml            # explicit deps, cdylib
├── Trunk.toml            # dist = ../dist, dev server :1420
├── index.html            # CSS links + rust entry + Tauri global loader
├── .cargo/config.toml    # wasm test runner
├── assets/styles/        # CSS ported verbatim (tokens/base/global)
└── src/
    ├── lib.rs            # crate root, #[wasm_bindgen(start)]
    ├── models.rs         # wire types + pure helpers (unit-tested)
    ├── pages/app.rs      # root component
    ├── components/       # drop_zone, file_card, progress_view, result_view, error_view
    ├── contexts/pdf_splitter.rs   # state + async actions
    └── services/         # tauri.rs (IPC) + commands.rs (typed wrappers)
```

`src-tauri/tauri.conf.json` uses the Trunk toolchain: `beforeDevCommand: "trunk
serve"`, `beforeBuildCommand: "trunk build"`, `devUrl: http://127.0.0.1:1420`,
`frontendDist: "../dist"`.

---

## 7. Known deviations & follow-ups

- **Mount target.** App mounts via `mount_to_body`; `index.html` keeps an unused
  empty `<div id="app">` (no CSS targets it — harmless; can be removed).
- **`assets/styles/main.css`** is an `@import` aggregator kept for parity;
  `index.html` links the three stylesheets directly, so `main.css` is
  unreferenced (harmless).
- **`AGENTS.md`** still references the removed `bun run` steps and should be
  updated to the Rust/Trunk commands above.
