# ADR-0001: Record architecture decisions

- Status: Accepted
- Date: 2026-10-08
- Phase: P00

## Context

Milestone 1 is delivered in 50 small phases, each reviewed and merged on its own ([plan §7](../milestone-1/implementation-plan.md#7-how-phases-work)). Many phases make decisions that later phases depend on: how the server is composed around `teta-wot`, which units the stage uses, how the web app talks to the server, how data is stored. Some of these will be revisited in milestone 2, when real hardware arrives, and again when the Tauri app is built.

If decisions live only in pull-request threads or chat, they are hard to find, and the reasons behind them get lost. The project goals also ask for every phase's decisions to be saved under `docs/`.

## Decision

Keep lightweight architecture decision records in `docs/adr/`:

- one decision per record, using the sections in [template.md](template.md): Context, Decision, Consequences, Alternatives considered;
- numbered across the whole project with four digits (`0001`, `0002`, …), never reused, with a short kebab-case title in the file name;
- written in the pull request of the phase that makes the decision, with the status *Accepted*. Approving the pull request accepts the ADR, and changes asked for in review are made in the same pull request;
- left unchanged after acceptance except for the status line. A changed decision gets a new ADR that supersedes the old one;
- listed in the index in [README.md](README.md).

What a phase built, how to try it, and any deviations from the plan go in its implementation notes (`docs/milestone-1/notes/`), not in ADRs.

## Consequences

- Anyone joining later, or a later phase, can find why something is the way it is without reading old pull requests.
- Each significant decision takes a little extra writing. The plan lists the ADRs it expects, so the cost is known in advance.
- Because ADRs are immutable, the history of a decision stays visible when it changes.

## Alternatives considered

- **Decisions only in pull-request descriptions.** No extra files, but the decisions would be scattered, tied to GitHub, and hard to supersede cleanly.
- **One running decisions log.** Simple, but a single file conflicts across branches and makes superseding an entry awkward.
- **The full MADR template** (decision drivers, pros and cons per option, links). More structure than decisions of this size need; our template keeps MADR's core sections.
