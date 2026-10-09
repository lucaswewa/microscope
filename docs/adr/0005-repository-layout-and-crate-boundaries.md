# ADR-0005: Repository layout and crate boundaries

- Status: Accepted
- Date: 2026-10-08
- Phase: P01

## Context

The repository will hold a Rust backend and launcher, a Vue web app, configuration files and documentation. On the Rust side, the work splits naturally into:

- pure computation (registration, camera–stage mapping, focus metrics, background detection, planners, mosaics);
- the simulator;
- the `teta-wot` Things that expose both;
- the server binary that composes everything;
- the launcher.

Milestone 2 replaces the simulator with real drivers but keeps the algorithms. Tests of the algorithms and of the simulator should be fast and shouldn't need a server.

## Decision

**Layout:**

```text
Cargo.toml, rust-toolchain.toml, rustfmt.toml   workspace, toolchain (1.98.1), formatting
crates/
  microscope-core/       algorithms, free of I/O
  microscope-sim/        simulator: specimens, optics, stage model
  microscope-things/     teta-wot Things, hardware interfaces, server-described UI model
  microscope-server/     binary: CLI, lifecycle, app routes, logging, embedded web app
  microscope-launcher/   binary: local supervisor
configs/                 server configuration files
web/                     the Vue 3 + TypeScript app (npm project, outside the Cargo workspace)
docs/                    plan, ADRs, implementation notes, reference-capture guide
```

**Dependency rules:**

| Crate | May depend on |
|---|---|
| `microscope-core` | external crates only |
| `microscope-sim` | `microscope-core` |
| `microscope-things` | `microscope-core`, `microscope-sim`, `teta-wot` |
| `microscope-server` | `microscope-things`, `teta-wot` |
| `microscope-launcher` | no internal crates, and not `teta-wot` |

- **`microscope-core`** depends on no internal crate and not on `teta-wot`. It does no I/O (files, network, clocks or threads of its own) and is deterministic.
- **`microscope-sim`** depends only on `microscope-core` among internal crates, and not on `teta-wot`. Time is injected rather than read from the system clock.
- **`microscope-things`** is the only library that depends on `teta-wot`. It connects the algorithms and the simulator to Things, and it defines the interfaces between Things.
- **`microscope-server`** composes the Things into a running server. It may gain a library target when its tests need one (P02).
- **`microscope-launcher`** doesn't depend on the other crates. Whatever it shares with the server goes through the sidecar contract (P44), not shared code, so a Tauri shell can follow the same contract later.

**Workspace-wide settings** are shared through `[workspace.package]` and `[workspace.lints]`: edition 2024, `rust-version = "1.98"`, the MIT licence, `publish = false`, `missing_docs = "warn"`, `unsafe_code = "forbid"`, and clippy's `all` group. CI turns warnings into errors. A crate that needs `unsafe` (perhaps the launcher, for Windows Job Objects) must justify the exception in an ADR.

**Line endings:** `.gitattributes` stores and checks out text files with LF on every platform.

## Consequences

- Algorithms and the simulator compile and test without `teta-wot`'s dependency tree, so their tests stay fast.
- Swapping the simulator for real hardware in milestone 2 touches `microscope-things` and configuration, not the algorithms.
- Each new type has to be placed in the right crate. Code that seems to need a forbidden dependency is a sign it belongs elsewhere.
- The layering is enforced by `Cargo.toml` alone. A forbidden dependency would show up in review, not in CI.

## Alternatives considered

- **One crate.** It would be simpler to start with, but boundaries would erode, every test would compile `teta-wot`, and the algorithms couldn't be reused without it.
- **More, smaller crates** (separate crates for planners, stitching, registration). That's premature: modules inside `microscope-core` give the same separation, and a module can become a crate later if its compile time or reuse calls for it.
- **The web app inside a Cargo crate** (for example built by a `build.rs`). It would couple the two toolchains. Instead, the server embeds a built `web/dist` (P12).
