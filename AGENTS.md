# Agent Notes — pdf-splitter

## Workspace Layout

This project is a **Cargo workspace** with strict crate boundaries:

```
.
├── Cargo.toml                 # Workspace root manifest
├── .clippy.toml               # Shared Clippy thresholds
├── .cargo/config.toml         # Shared rustflags & target-dir
├── rustfmt.toml               # Shared rustfmt config
├── crates/
│   └── pdf-split-core/        # Pure Rust (NO Tauri dependency)
├── src-tauri/                 # Tauri app (thin wrapper only)
└── src/                       # Leptos frontend (Rust → WASM, built with Trunk)
```

**Rules of the road** (enforced by `clippy::pedantic` + `clippy::nursery`):

1. **NEVER** add `tauri`, `tauri-build`, or any Tauri-related crate as a
   dependency to any member under `crates/`.  The pure crates must stay
   pure so they can be unit-tested, fuzz-tested, or `miri`-tested in
   isolation from the desktop shell.
2. **NEVER** put business logic, algorithms, validation rules, or data
   transformations in `src-tauri/src/`.  Tauri files should only contain
   command handlers, plugin setup, and IPC glue.
3. **NEVER** use `.unwrap()` / `.expect()` in non-test code.  All public
   functions return `Result<T, E>` with descriptive error types.
4. **NEVER** use `{ workspace = true }` to inherit dependencies.  Every
   crate ships its own dependency versions explicitly so a member can
   be lifted out of the workspace and built in isolation.
5. **NEVER** write `unsafe` — the workspace forbids it via
   `[workspace.lints.rust] unsafe_code = "forbid"`.

## Required Verification

Run **all** of the following before opening a PR or declaring a task
complete.  Every command must exit 0.

```bash
# Format
cargo fmt --all -- --check

# Lint (treats every warning as an error)
cargo clippy --workspace --all-targets -- -D warnings

# Tests (must pass for every member crate)
cargo test --workspace

# Frontend (Leptos → WASM).  The `src/` crate is excluded from the Cargo
# workspace (its wasm-only deps would break host builds), so it is validated
# separately with Trunk.
cd src
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown --all-targets   # -D warnings via .cargo/config
cargo test  --target wasm32-unknown-unknown                  # wasm-bindgen-test-runner + Node
trunk build                                                  # emits ../dist
```

The pure-logic crate can additionally be run through Miri for extra
confidence (no FFI, no `unsafe`, so it is miri-compatible):

```bash
cargo +nightly miri test -p pdf-split-core
```

## Adding a New Crate

1. `cargo new --lib crates/<name>`
2. Add the path to `[workspace.members]` in root `Cargo.toml` (the
   `crates/*` glob picks it up automatically — no edit needed there).
3. Declare **all** dependency versions explicitly in the new crate's
   `Cargo.toml` (no `workspace = true` indirection).
4. Add `#![warn(missing_debug_implementations)]` and the same
   `#![deny(clippy::all, clippy::pedantic, clippy::nursery, unsafe_code)]`
   header used by sibling crates.
5. Write at least one unit test per public function.  Critical logic
   (calculations, parsing, validation) must have tests for the normal
   range, boundary values, invalid inputs, and known reference values.
6. Add a `pub mod test_utils` (#[doc(hidden)]) if external test crates
   need to share fixtures — see `pdf-split-core` for the pattern.

## Cross-Crate Test Fixtures

When `src-tauri` tests need to construct synthetic PDFs, they import
the fixtures from `pdf_split_core::test_utils` instead of duplicating
the lopdf boilerplate.  That module is `#[doc(hidden)] pub` on
purpose — it is part of the public API *only* so external test
crates can reuse it.
