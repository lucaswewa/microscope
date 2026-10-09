# ADR-0004: Depend on `teta-wot` v0.1.0 without upstream changes

- Status: Accepted
- Date: 2026-10-08
- Phase: P00

## Context

The backend is built on [`teta-wot`](https://github.com/lucaswewa/teta-wot), the project owner's Rust framework for W3C Web of Things servers. Relevant facts, checked against the code at tag `v0.1.0`:

- It is MIT-licensed and isn't published to crates.io (`publish = false`), so applications depend on it through Git.
- The crate applications use is `teta-wot` (library `teta_wot`); the other workspace crates sit behind it.
- Its guide is out of date in places. The introduction and a comment in the workspace `Cargo.toml` say the project has no licence, and the "Installing" page names an older `wot` crate and repository. Where the guide and the code disagree, we follow the code.
- It has no hook for adding routes to the served router. This app needs routes of its own: the web app at `/`, log and data downloads. Planning considered adding such a hook upstream (D6 in the [plan](../milestone-1/implementation-plan.md#3-decisions-made-while-planning)).

## Decision

- Depend on `teta-wot` through a workspace dependency pinned to the release tag:

  ```toml
  teta-wot = { git = "https://github.com/lucaswewa/teta-wot", tag = "v0.1.0" }
  ```

  Features (`testing`, `ndarray`, …) are enabled per crate as they're needed. `Cargo.lock` records the exact commit.
- **No changes to `teta-wot` in milestone 1.** Gaps are worked around in this repository. Each workaround gets an ADR or a note, and goes on the list of upstream candidates below. The serve-loop workaround is ADR-0006 (P02).
- Upgrading `teta-wot` is a deliberate change of its own: it moves the tag in its own pull request, which runs the full test suite and checks the workarounds still match `teta-wot`'s behaviour.

**Upstream candidates** (updated as phases find more):

- a hook to add routes, or a fallback service, to `ThingServer`'s router, which would retire the serve-loop workaround;
- a public mDNS advertiser that works with an app-owned serve loop;
- guide fixes: the licence statement and the `wot` naming.

## Consequences

- Builds are reproducible, and CI needs no local checkout of `teta-wot`.
- `teta-wot` stays stable while milestone 1 is built on it.
- The workarounds cost some maintenance, and must be re-checked whenever the tag moves.

## Alternatives considered

- **A path dependency on `../teta-wot`.** Convenient for co-development, but builds would depend on the state of a local checkout, and CI couldn't build without one.
- **Adding the router hook upstream first.** It would remove the main workaround, but the owner chose to keep `teta-wot` unchanged during milestone 1.
- **Vendoring `teta-wot` into this repository.** It would duplicate the code and cut it off from upstream fixes.
