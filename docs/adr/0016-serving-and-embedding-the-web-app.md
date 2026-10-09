# ADR-0016: Serving and embedding the web app

- Status: Accepted
- Date: 2026-10-09
- Phase: P12

## Context

- People should be able to run the microscope with one program: the server, serving both the API and the web app. The launcher (P44) and the Windows packages (P49) also want a single file.
- The web app is built by Vite into `web/dist`: an `index.html`, content-hashed files under `assets/`, and a favicon. It routes in the URL's hash (ADR-0010), so the server only needs `/`, never a route per page.
- Development uses Vite's server, which reloads on every change and forwards `/api` to a running server (ADR-0009). That must keep working.
- The server answers its own routes in front of `teta-wot`'s router, with no changes to `teta-wot` (ADR-0004, ADR-0006).
- The Rust CI job builds the server without the web app.

## Decision

- **The server serves the app** at `/`, `/index.html`, `/assets/{*path}` and `/favicon.svg`, from its own router in front of `teta-wot`'s. Any other path is still `teta-wot`'s, and a missing asset answers `{"detail": "Not Found"}`, as `teta-wot` does.
- **The files come from `web/dist`, through `rust-embed`:**
  - **Release builds embed it,** so the binary is self-contained.
  - **Debug builds read it from disk,** so `cargo run` serves the latest web build without recompiling.
  - **`--webapp-dir <path>`** serves another folder instead.
- **Without a build** (`web/dist` missing when the server was compiled), `/` shows a placeholder page that says how to build the app, and links to the API and its documentation. The server still compiles and runs.
- **Caching:**
  - `index.html` is never cached (`no-store`), so a new server's app loads at once.
  - Hashed assets are cached for a year as immutable.
  - The favicon is revalidated (`no-cache`).
- **The build order** is the web app first, then the server. A build script asks Cargo to rebuild the server when `web/dist` changes. CI builds them in that order and runs the end-to-end and visual tests against the release server.
- **The end-to-end tests test the served app,** everywhere. Locally, Playwright builds the web app and runs a debug server with `--webapp-dir web/dist`. CI runs the release build, which has the app embedded.
- **Paths that could leave the build's folder are refused.** Only plain file names are accepted: no `..`, no root or drive.
- **The dependency:** `rust-embed` 8.12 (MIT), pinned below 8.13, which was too new for the two-week rule. The crates it brings are MIT, Apache-2.0 or Unlicense, with `unicase` pinned to 2.9.0 for the same reason.

## Consequences

- One binary serves everything. The app it serves is always the one it was built with, so the two never drift apart.
- A release build needs the web app built first. Building the server alone gives the placeholder, which says so.
- While `web/dist` is missing, the build script runs on every build, and Cargo rebuilds the server crate. That costs a few seconds, and stops once the app is built.
- The end-to-end tests now exercise the serving path that people will use, rather than `vite preview`. They need Rust to build the server.
- The release binary grows by the app's size, about 300 KB today.

## Alternatives considered

- **Shipping `web/dist` beside the binary,** read from disk at run time. That makes two things to install and keep in step. `--webapp-dir` still allows it.
- **`include_dir`,** or our own build script generating `include_bytes!`. `rust-embed` does the same, with the disk-reading debug mode built in.
- **`tower-http`'s `ServeDir`.** It reads from disk only, so it can't serve an embedded build.
- **Serving the app from a sub-path such as `/app/`.** The root is what people type, and the API already has its own prefix.
