# ADR-0035: Release packaging and third-party notices

- Status: Accepted
- Date: 2026-10-15
- Phase: P49

## Context

- **Milestone 1 ends with something to hand to someone:** a Windows computer without Rust, Node or a clone of the repository should run the microscope (plan §2).
- **The launcher** (ADR-0033) runs the server beside it, and the release server embeds the web app (ADR-0016). Together, the two binaries are the whole application.
- **The project is MIT-licensed,** and its binaries include about 180 Rust crates and 31 npm packages. Their licences, mostly MIT and Apache-2.0, ask for their notices to go with copies.
- **Two pieces are visible in the app:** Material Symbols' icons (Apache-2.0) and OpenSeadragon's viewer (BSD-3-Clause).
- **`teta-wot`'s crates** declare no licence in their manifests, though the repository is MIT-licensed (ADR-0004).

## Decision

- **A zip,** `microscope-<version>-windows-x86_64.zip`, with one folder:

  | File | What |
  |---|---|
  | `microscope-launcher.exe` | The launcher, which a user runs |
  | `microscope-server.exe` | The server, with the web app embedded |
  | `configs\` | The default configurations |
  | `README.md` | Its user guide comes first |
  | `LICENSE`, `NOTICE` | The project's licence, and the third-party notices people should see |
  | `THIRD-PARTY-RUST.txt` | Every Rust crate in the binaries, under each licence's text |
  | `THIRD-PARTY-NPM.txt` | Every npm package the web app bundles, with its licence's text |

  No installer: the folder runs where it's unzipped, and the launcher keeps the user's data in `%LOCALAPPDATA%\Microscope` (ADR-0033).
- **The Rust licences** come from [cargo-about](https://github.com/EmbarkStudios/cargo-about) 0.9.2, offline, from the sources Cargo already has ([`release/about.toml`](../../release/about.toml)).
  - **It accepts** MIT, Apache-2.0, the BSD, Zlib, 0BSD, Unicode and Boost licences, and the Unlicense, preferring them in that order where a crate offers a choice.
  - **It fails** on any other licence, so a new dependency with an unexpected one stops the release.
  - **Unpublished crates are left out:** ours, and `teta-wot`'s, which `NOTICE` names.
- **The npm licences** come from the web app's build: a Vite plugin lists the packages its modules come from, with their licence files, as `third-party-licences.txt`. It includes development dependencies the bundle uses, such as the icons, and leaves out tools that aren't in it.
- **`NOTICE`** names Material Symbols, OpenSeadragon and `teta-wot`, and points to the two full lists.
- **The release workflow** ([`release.yml`](../../.github/workflows/release.yml)) builds the web app, the binaries, the notices and the zip, through [`release/package.ps1`](../../release/package.ps1), which runs locally too.
  - **Before keeping it,** it unzips the zip and checks its launcher starts a server that answers.
  - **A tag `v*`** publishes the zip as a GitHub release.
  - **A manual run,** or a pull request that changes the packaging, keeps it as an artifact.

## Consequences

- **A release is a tag:** CI builds, checks and publishes it, and nothing is built by hand.
- **The notices follow the dependencies:** a new crate or package appears in the next release's lists, and an unexpected licence fails it.
- **Unsigned binaries:** Windows SmartScreen may warn on first run. Code signing needs a certificate, a later decision.
- **The web app's build** carries `third-party-licences.txt`, so the server embeds it too. It isn't served.

## Alternatives considered

- **An installer (MSI or MSIX):** Start-menu shortcuts and an uninstaller, but signing and more to maintain. A zip is enough to hand to someone for milestone 1, and the Tauri app will bring its own installer.
- **cargo-deny's licence report, or `cargo-license`:** cargo-deny checks licences but doesn't write the texts; `cargo-license` lists the licences without the texts the notices need.
- **license-checker or a rollup licence plugin for npm:** another dependency, and license-checker lists what's installed rather than what's bundled.
- **The full licence texts in `NOTICE`:** one long file, written by hand, which would fall behind the dependencies.
