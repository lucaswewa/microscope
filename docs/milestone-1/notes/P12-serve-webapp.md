# P12: Serve the web app from the backend

- Status: In review
- Pull request: #NN
- ADRs: [ADR-0016](../../adr/0016-serving-and-embedding-the-web-app.md)
- Spec: [phases.md#p12](../phases.md#p12)

## Summary

`microscope-server` now serves the web app as well as the API, so one program is the whole microscope:

- **Release builds** embed the app's build (`web/dist`).
- **Debug builds** read it from disk.
- **`--webapp-dir`** serves another folder.
- **Without a build,** `/` shows a placeholder page that says how to build one.

The page is never cached and the hashed assets are cached for good. A new CI job builds the web app, then a release server with it embedded, and runs the end-to-end and visual tests against that server.

## What was built

| Where | What |
|---|---|
| `crates/microscope-server/src/webapp.rs` | The web app's routes: `/`, `/index.html`, `/assets/{*path}` and `/favicon.svg`, from the embedded build or a folder, with caching, media types, the placeholder page, and paths kept inside the build |
| `crates/microscope-server/src/cli.rs` | `--webapp-dir <PATH>`, and the app's routes merged with the server's own, in front of `teta-wot`'s |
| `crates/microscope-server/build.rs` | Rebuilds the server when `web/dist` changes |
| `Cargo.toml` | `rust-embed` 8.12 |
| `web/public/favicon.svg`, `web/index.html` | The favicon: Material Symbols' "biotech" (a microscope) in the accent colour |
| `web/playwright.config.ts`, `web/tests/e2e/connection.spec.ts` | The end-to-end tests run against the app as the server serves it, on port 5098. `MICROSCOPE_SERVER` names a server binary to use instead of building one |
| `.github/workflows/ci.yml` | An **End to end** job: the web app, then the release server, then Chromium and the whole Playwright suite, uploading the results if it fails |
| `crates/microscope-server/tests/webapp.rs` | The tests below |
| `README.md`, `CLAUDE.md` | Serving the app, and the build order for a release |

## How to try it

From a clean checkout:

```powershell
cd web
npm ci
npm run build
cd ..
cargo build --release -p microscope-server
target/release/microscope-server.exe -c configs/simulation.json
```

1. Open <http://127.0.0.1:5000/>. The app loads from the server, and connects to it: the rail shows the host name.
2. Stop it, and start a debug build without the web app built (`cargo run -p microscope-server -- -c configs/simulation.json --webapp-dir nowhere`). `/` shows the placeholder.

## Design notes and deviations from the plan

ADR-0016 records the decisions. The details:

- **Embedding:**
  - `rust-embed` with `allow_missing`, so the server compiles without a web build and shows the placeholder.
  - In debug builds `rust-embed` reads the folder at run time.
  - In release builds it embeds the files: with `web/dist` moved aside, the release binary still served the app.
- **The build script** asks Cargo to watch `web/dist`, because `rust-embed` can't tell Cargo about files added to the folder. While the folder is missing, the script runs on every build.
- **Routes and caching:**

  | Route | Media type | `Cache-Control` |
  |---|---|---|
  | `/`, `/index.html` | `text/html; charset=utf-8` | `no-store` |
  | `/assets/*.js`, `.css`, `.svg`, `.png`, `.woff2`, `.json`, `.map` | by extension, otherwise `application/octet-stream` | `public, max-age=31536000, immutable` |
  | `/favicon.svg` | `image/svg+xml` | `no-cache` |

  The spec's "favicon" is an SVG. The app had none, so P12 adds one. `/favicon.ico` stays `teta-wot`'s 404, since the page links to the SVG.
- **Paths:** only plain file names under the build are served. A decoded `..`, a root or a drive prefix is refused with a 404. The tests cover `..%2F`, `%2E%2E%2F` and `..%5C`.
- **The end-to-end tests now test the served app,** not `vite preview`.
  - **Locally,** Playwright builds the web app and runs `cargo run … --webapp-dir web/dist` on port 5098. Port 4173 is no longer used.
  - **In CI,** `MICROSCOPE_SERVER` points at the release build, and the reconnect test's second server uses the same binary.
  - Against the release build, all 15 passed locally.
- **The CI job runs the visual baselines too,** on GitHub's Windows runners. If font rendering there differs from the machine that made the baselines by more than the 1% tolerance, they'll fail, and the uploaded results will show by how much.
- **Dependencies:**
  - `rust-embed`, `rust-embed-impl` and `rust-embed-utils` are pinned to 8.12.0: 8.13.0 is two days old.
  - `unicase` is pinned to 2.9.0, since 2.10.0 is four days old.
  - The other new crates (`walkdir`, `sha2`, `mime_guess` and their dependencies) are at least five weeks old, and all are MIT, Apache-2.0 or Unlicense.
- **Size:** 469 changed lines of hand-written code, within M's budget of 600.

## Tests

- **Rust** (6 new, in `tests/webapp.rs`):
  - the page uncached and the assets cached for good, with their media types;
  - a missing asset answered as `teta-wot` would;
  - three encodings of `..` refused;
  - the placeholder without a build;
  - the app's routes in front of the API, with the API still answering;
  - `--webapp-dir` through the binary.

  Mutation check: without the path check, the traversal test fails. Every server test also passes with `web/dist` absent, as in the Rust CI job.
- **End to end:** the 15 Playwright tests pass against the served app, from a debug build with `--webapp-dir` and from the release build with the app embedded.
- **Web:** the 256 unit tests are unchanged and pass.

## Follow-ups and known gaps

- P44 (launcher) and P49 (packaging) build on the single binary.
- If the visual baselines don't hold on CI's runners, they may need baselines made there, or a looser tolerance there only.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The favicon is a Material Symbol (Apache-2.0, in NOTICE).
