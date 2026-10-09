# Architecture decision records

An ADR records one significant decision: the context, what was decided, its consequences, and the alternatives that lost. [ADR-0001](0001-record-architecture-decisions.md) explains why we keep them.

## Process

1. Write the ADR in the pull request of the phase that makes the decision. Copy [template.md](template.md) to `NNNN-short-title.md`, using the next free number.
2. Give it the status *Accepted*. Approving the pull request accepts the ADR. If the review changes the decision, the ADR is edited in the same pull request. *Proposed* is only for an ADR written ahead of a decision, to collect review on it.
3. After acceptance, an ADR isn't edited except for its status line. To change a decision, write a new ADR and mark the old one *Superseded by ADR-NNNN*.
4. Add a row to the index below.

ADRs say *why*. What was built, and how, goes in the phase's [implementation notes](../milestone-1/notes/README.md).

## Index

| ADR | Title | Status | Phase |
|---|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted | P00 |
| [0002](0002-independent-implementation-of-openflexure-inspired-behaviour.md) | Independent implementation of OpenFlexure-inspired behaviour | Accepted | P00 |
| [0003](0003-milestone-1-platform-windows-x86-64.md) | Milestone-1 platform: Windows x86_64 | Accepted | P00 |
| [0004](0004-depend-on-teta-wot-v0-1-0-without-upstream-changes.md) | Depend on `teta-wot` v0.1.0 without upstream changes | Accepted | P00 |
| [0005](0005-repository-layout-and-crate-boundaries.md) | Repository layout and crate boundaries | Accepted | P01 |
| [0006](0006-own-the-server-lifecycle-around-teta-wots-runtime.md) | Own the server lifecycle around `teta-wot`'s runtime | Accepted | P02 |
| [0007](0007-api-prefix-and-openflexure-mirrored-names.md) | API prefix `/api/v1` and OpenFlexure-mirrored names | Accepted | P02 |
| [0008](0008-server-logging-and-log-endpoints.md) | Server logging and log endpoints | Accepted | P03 |

The [implementation plan](../milestone-1/implementation-plan.md#adr-plan) lists the ADRs planned for later phases.
