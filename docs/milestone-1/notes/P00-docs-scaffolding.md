# P00: Docs scaffolding and foundational ADRs

- Status: In review
- Pull request: opened from branch `m1/p00-docs-scaffolding`
- ADRs: [ADR-0001](../../adr/0001-record-architecture-decisions.md), [ADR-0002](../../adr/0002-independent-implementation-of-openflexure-inspired-behaviour.md), [ADR-0003](../../adr/0003-milestone-1-platform-windows-x86-64.md), [ADR-0004](../../adr/0004-depend-on-teta-wot-v0-1-0-without-upstream-changes.md)
- Spec: [phases.md#p00](../phases.md#p00)

## Summary

Sets up the documentation system for the milestone and records the four decisions that frame it. It also commits the approved plan, which until now existed only in the working tree, and replaces the one-line README.

## What was built

| Path | What |
|---|---|
| `README.md` | Rewritten: what the project is, its status, how it relates to `teta-wot` and OpenFlexure, links to the docs, licence |
| `.github/pull_request_template.md` | Summary, how to try it, and the Definition-of-Done checklist from [plan §7](../implementation-plan.md#7-how-phases-work) |
| `docs/README.md` | Index of all documentation |
| `docs/milestone-1/goals.md` | Moved from the repository root (plan §11, point 12) |
| `docs/milestone-1/implementation-plan.md`, `phases.md` | The approved plan and phase specifications |
| `docs/adr/README.md`, `template.md` | ADR process and index |
| `docs/adr/0001` … `0004` | The foundational ADRs |
| `docs/milestone-1/notes/README.md`, `template.md` | Notes index and template |

## How to try it

- Read [docs/README.md](../../README.md) on GitHub and follow its links. Every link should resolve.
- Once this is merged, opening a new pull request pre-fills the template.

## Design notes and deviations from the plan

- **ADR status.** The plan said ADRs become *Accepted* "on merge". Since a merged pull request can't be edited, approving a pull request now accepts the ADRs in it; they are written as *Accepted*, and edited in the same pull request if review changes them. ADR-0001 and the ADR README describe this. The plan's §7 and change log are updated to match.
- **Phase status.** For the same reason, each phase's pull request marks that phase *In review* in the plan's overview and in its notes, and the next phase's pull request marks it *Done*.
- **`docs/README.md`** was first written with the plan. This phase extends it rather than creating it.
- **The plan's header** now says it's approved and that it's a living document.
- **Commit author.** No git identity is configured on the development machine, so commits use the identity of the repository's initial commit (`Lucas Wang <58446606+lucaswewa@users.noreply.github.com>`) through per-command `-c` options. No git configuration was changed.

## Tests

Documentation only. All relative links in `README.md` and `docs/` were checked with a script: every target file exists, and every `#anchor` matches a heading or an explicit anchor.

## Follow-ups and known gaps

- The GitHub CLI (`gh`) isn't installed, so pull requests can't be opened from the command line yet. With `gh` installed and authenticated, later phases can open their own pull requests.
- P01 adds the CI workflow. Until then, this pull request runs no checks.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
