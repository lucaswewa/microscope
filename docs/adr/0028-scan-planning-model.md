# ADR-0028: Scan planning model

- Status: Accepted
- Date: 2026-10-13
- Phase: P34

## Context

- **Four workflows** visit sites with their own patterns (plan D3): a raster, a snake, the C-Chip's grid, and the Histo smart spiral. The spiral follows the sample and skips background (P37).
- **The scan engine** (P35) moves the stage, autofocuses, maybe takes a z-stack (P36), and captures at each site. It needs to know where to go next, and where to start focusing from.
- **Planning is the part with the most cases to test,** so it should be pure: no stage, camera or clock. The workspace's rules put such code in `microscope-core` (ADR-0005).

## Decision

- **`microscope_core::scan`,** with no dependencies, as the rest of `microscope-core`.
- **Sites** are stage positions in x and y, in steps (`[i64; 2]`), as everything else on the stage is (ADR-0019).
- **A `Planner`** is asked for `next()`, a site and the z to start focusing from. It's told `visited(z, imaged, focused)`: the z where the stage ended, whether an image was taken, and whether autofocus succeeded.
  - It keeps its `visits()`, and says why it `finished()`.
  - **The visit is the one `next` gave,** so a planner can't be told about the wrong site.
- **The planners:**
  - **`RegularGrid`,** raster or snake, with counts in x and y. The C-Chip pattern is a snake of 5 × 7 at an overlap of 0.1 (`RegularGrid::c_chip`).
  - **`SmartSpiral`,** with a maximum range, which follows the sample and fills the gaps at its edge. It reports `ReachedRange` or `SurroundedByBackground`.
- **The step** comes from the camera–stage mapping and an overlap (`Step::from_overlap`), each axis from the image's motion that moves the stage most along it. The signs are kept, so the grid runs right and down in the image. `Step::equal` makes both steps the smaller.
- **The z estimate belongs to each planner:**
  - **a grid's** comes from the focused site fewest moves away, the latest if several tie;
  - **the spiral's** comes from the lowest of the focused sites within a step, else of the nearest.
- **Errors,** `PlanError`, say what's wrong: an overlap outside 0 to 1, a step of 0, or a count of 0.
- **The behaviour spec,** in the P34 notes, is the reference. `examples/plan.rs` draws plans as SVG, for review.

## Consequences

- **The engine** (P35) only drives a `Planner`: it needn't know which pattern it follows. A new pattern is a new planner.
- **The tests are fast and deterministic.** Background is a function of the site, so a scan over any imagined sample can be scripted.
- **The z estimate's rule is fixed per planner.** A workflow that wants another must wrap its planner.
- **Plans aren't saved.** A scan's folder will hold its settings and visits (P35), from which a plan can be made again.

## Alternatives considered

- **Planning inside the workflow Things,** as OpenFlexure's workflows set up their planners. Tests would then need a server and hardware, or fakes of them.
- **Paths computed whole, up front.** That's right for grids, but the spiral depends on what it finds, so it needs to be told about each site.
- **Positions in µm.** A stage's scale may be unknown (ADR-0019), and the mapping is in steps.
