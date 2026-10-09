# P01: Rust workspace and CI

- Status: In review
- Pull request: to be linked once opened
- ADRs: [ADR-0005](../../adr/0005-repository-layout-and-crate-boundaries.md)
- Spec: [phases.md#p01](../phases.md#p01)

## Summary

Creates the Cargo workspace with the five crates the plan uses, pins the toolchain to `teta-wot`'s, adds `teta-wot` v0.1.0 as a Git dependency, and adds a Windows CI job that checks formatting, lints, tests and docs. Nothing runs yet beyond the two binaries printing their versions.

## What was built

| Path | What |
|---|---|
| `Cargo.toml` | Workspace: resolver 3; shared package settings (edition 2024, `rust-version = "1.98"`, MIT, `publish = false`); internal crates and `teta-wot` (tag `v0.1.0`) as workspace dependencies; workspace lints (`missing_docs`, `unsafe_code = "forbid"`, clippy `all`) |
| `rust-toolchain.toml` | Rust 1.98.1, minimal profile, rustfmt and clippy, MSVC target: the same as `teta-wot` |
| `rustfmt.toml` | `style_edition = "2024"`, as in `teta-wot` |
| `.gitattributes` | LF line endings for text files on every platform |
| `crates/microscope-core`, `microscope-sim`, `microscope-things` | Library skeletons whose crate docs state each crate's role, with dependencies following ADR-0005 |
| `crates/microscope-server`, `microscope-launcher` | Binaries that print their name and version |
| `Cargo.lock` | Pins `teta-wot` to commit `135f9ff4` (tag `v0.1.0`) |
| `.github/workflows/ci.yml` | The `rust` job on `windows-latest`: fmt, clippy (`-D warnings`), test, doc (`-D warnings`), all `--locked` |
| `.gitignore` | `node_modules/`, `web/dist/`, `.microscope/` (development data), and the reference-screenshot folders. Also includes the owner's pending trailing blank line |
| `README.md` | A Development section: prerequisites and the commands CI runs |

## How to try it

```powershell
rustup toolchain install          # installs 1.98.1 from rust-toolchain.toml, if needed
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p microscope-server -- --version     # microscope-server 0.1.0
cargo run -p microscope-launcher -- --version   # microscope-launcher 0.1.0
```

## Design notes and deviations from the plan

- **The internal dependencies are declared now,** even though the crates are empty: `microscope-sim` → core; `microscope-things` → core, sim and `teta-wot`; `microscope-server` → things and `teta-wot`. Declaring them puts the layering from ADR-0005 in place from the start, and makes CI compile `teta-wot` at the pinned tag with the pinned toolchain, which shows early that the dependency works.
- **`.gitattributes`** wasn't in the phase spec. It was added because `core.autocrlf` on the development machine was converting the LF files written for P00, and the warnings on every commit would hide real problems. ADR-0005 records it.
- **The binaries print their version whatever the arguments.** P02 replaces the server's `main` with a real command line, and P45 replaces the launcher's.
- **CI uses the same actions as `teta-wot`'s workflow** (`actions/checkout@v7`, `Swatinem/rust-cache@v2`, `rustup toolchain install` reading `rust-toolchain.toml`), so the two repositories stay alike.

## Tests

There are no unit tests yet: the crates are empty. CI runs every check (fmt, clippy, tests, docs), so the first tests, in P02, run without workflow changes. Locally, all four checks passed on Rust 1.98.1, and both binaries print their versions.

## Follow-ups and known gaps

- The `web` CI job arrives with P04, and the `e2e` job with P12.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
