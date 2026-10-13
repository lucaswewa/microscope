# P34: Scan planners

- Status: Done
- Pull request: [#46](https://github.com/lucaswewa/microscope/pull/46)
- ADRs: [ADR-0028](../../adr/0028-scan-planning-model.md)
- Spec: [phases.md#p34](../phases.md#p34)

## Summary

The scan planners are built: pure code in `microscope-core` that says where a scan goes next. The scan engine (P35) and the workflows (P35, P37) will drive them.

- **The grid's step** comes from the field of view, less the overlap, turned into stage steps through the camera–stage mapping.
- **A regular grid:** snake or raster, with counts in x and y. The C-Chip pattern is a snake grid of the C-Chip's size.
- **The smart spiral:**
  - it grows out from the start around the sample, and skips the background;
  - it fills the gaps at the sample's edge;
  - it stays within a maximum range, and can make the x and y steps equal;
  - it says why it finished.
- **A z to start focusing from,** taken from nearby sites already in focus.
- **`examples/plan.rs`** draws planned paths as SVG, for review.

## Behaviour spec

Written before the code, in our own words, from what OpenFlexure's scan planners do.

### What a planner does

A scan visits **sites**: positions of the stage in x and y, in steps.

- **It's asked for the next site,** and a z to start focusing from.
- **It's told what happened there:** the z the stage ended at, whether an image was taken (a workflow may skip background), and whether autofocus succeeded.
- **When no sites are left,** it has finished, and it says why.

### The grid's step

1. **From the mapping and an overlap:**
   - The camera–stage mapping turns a move of the image in pixels into stage steps, for the frames' size it was calibrated at, w × h.
   - **A step across** is w × (1 − overlap) pixels to the right. **A step down** is h × (1 − overlap) pixels down. Each is turned into stage steps.
2. **The aligned axes:**
   - **Usually:** if the step across moves the stage more in x than in y, `dx` is its x, and `dy` is the step down's y.
   - **A camera turned a quarter:** otherwise, `dx` is the step down's x, and `dy` is the step across's y.
3. **Rounding and signs:**
   - Both are rounded to whole steps.
   - They keep their signs, so the grid runs right and down in the image, whichever way the camera is turned or flipped.
   - **An error:** a step of 0, or an overlap outside 0 to 1.
4. **Equal distances,** an option: both steps take the smaller of the two sizes, keeping their signs.

### Regular grids: raster and snake

1. **The sites:** `x_count` columns by `y_count` rows, the start being the first. Column i, row j is at start + (i·dx, j·dy).
2. **The order:**
   - **Raster:** row by row, each row in the same direction.
   - **Snake:** every other row reversed, so each move is to a neighbour.
3. **Known from the start:** no sites are added along the way.
4. **Finished** once every site has been visited: "every planned site was visited".
5. **An error:** a count of 0.

### The C-Chip pattern

A snake grid of 5 columns by 7 rows, at an overlap of 0.1, from the start as its corner: the size of a C-Chip's well. Its workflow (P37) takes a z-stack at each site, without autofocus.

### The smart spiral

1. **It starts with only the start site.** The sites it adds are either **main sites**, on the grid, or **between sites**, halfway between two grid sites.
2. **After a main site where an image was taken** (sample), each of its four neighbours, at ±dx and ±dy, is considered:
   - **Already planned,** or **visited with an image:** nothing is added.
   - **Visited without an image** (background): the site halfway between the two is added, to fill the gap at the sample's edge.
   - **Further from the start than the maximum range** (the straight distance, in steps): it's left out, and noted.
   - **Otherwise:** it's added as a main site.
3. **After a main site without an image** (background), for each neighbour visited with an image, the site halfway between them is added.
4. **Halfway** is the mean of the two positions, rounded down to whole steps.
5. **Between sites add nothing** when they're visited.
6. **The order:**
   - **After each main site,** the remaining sites are sorted:
     - between sites first;
     - then the fewest grid moves from where the stage is, counting the larger of the moves in x and in y;
     - then the fewest grid moves from the start, counted the same way;
     - then the shortest straight distance from where the stage is.
   - **After a between site,** the order stays as it is, so the spiral doesn't turn back.
7. **Finished** when nothing is left:
   - "it reached its maximum range", if a neighbour was ever left out for being too far;
   - else "the scanned area is surrounded by background".

### The z to start focusing from

None, until a site's autofocus has succeeded. After that, the sites where focus succeeded are considered.

- **Regular grids:** the one fewest grid moves away, counting the moves in x and in y together. If several tie, the latest of them.
- **The smart spiral:**
  - **Near sites:** those within one step in both x and y. Of these, the lowest z is chosen, since a z-stack searches up from where it starts (P36), and starting too high costs more. If several tie, the latest of them.
  - **Without a near site:** the ones the shortest straight distance away. Of those, the lowest z, and of those, the latest.

### Where it differs from OpenFlexure

- **The step:**
  - **A step of 0 is an error.** It would revisit one site for ever.
  - **The steps keep their signs,** so a flipped camera still scans right and down in the image.
- **The C-Chip pattern** is named in the planners, rather than only in its workflow's settings.
- **Why it finished** is a value (`Finished`), not a sentence. The workflows say it in their own words.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/scan.rs` | The parts of a plan: `Site`, `Step` (with `from_overlap` and `equal`), `Visit`, `Finished` and `PlanError`. The `Planner` trait. `RegularGrid` (raster, snake and `c_chip`), and `SmartSpiral` |
| `crates/microscope-core/examples/plan.rs` | Draws a snake grid, and the spiral over a sample of two disks, as SVG |
| `crates/microscope-core/tests/scan.rs` | The planners' tests |

## How to try it

```powershell
cargo run -p microscope-core --example plan -- .
```

It writes `snake.svg` and `spiral.svg` to the current folder. Open them in a browser:

- **The snake** covers its 8 × 6 grid row by row, turning at each end.
- **The spiral** grows out from the middle of the left disk, around the sample:
  - images are purple, background grey;
  - the gaps filled at the edge are orange;
  - it stops at its range of 1,000 steps, before the right disk's far side, and says `ReachedRange`.

```powershell
cargo test -p microscope-core --test scan
```

## Design notes and deviations from the plan

- **One trait, two planners.** A `Planner` is asked for the next site and told what happened there, so the engine (P35) can drive any of them. The visit is always the one `next` gave, so it can't be told about the wrong site (ADR-0028).
- **The C-Chip pattern** is `RegularGrid::c_chip`, a snake of `C_CHIP_COUNTS` (5 × 7), with `C_CHIP_OVERLAP` (0.1) for its workflow to use. Its z-stacks and the lack of autofocus are its workflow's (P37).
- **"Equal distances"** is `Step::equal`, applied to the step before the spiral is made.
- **The steps keep their signs:** a flipped camera still scans right and down in the image. A step of 0 is refused, since it would never move.
- **Between sites first** is in the order, as the spec has it, but it never changes it: a between site is always half a move from the site just visited, and every main site at least one. So no test can tell it's there (see the mutation checks).
- **The spiral's z** returns only the z, so "the latest of several with the lowest z" makes no difference to it. It does for a grid, whose ties can have different z.
- **Size:** 842 changed lines of hand-written code against M's budget of 600, 40% over, and above my estimate of 550–700. Asked at 815 lines, the owner chose to keep one pull request. The tests that the mutation checks called for then added 27.

  | Part | Lines |
  |---|---|
  | `scan.rs` | 383: 289 code, 57 comments, 37 blank |
  | `tests/scan.rs` | 378 |
  | `examples/plan.rs` | 80 |
  | `lib.rs` | 1 |

## Tests

`tests/scan.rs` (9 tests). Background is a function of the site, so each sample is scripted.

- **The step:**
  - 820 × 616 frames at 0.35 overlap through a 1.12 steps-per-pixel mapping: 597 × 448;
  - flipped, it's −597;
  - turned a quarter, the axes swap;
  - `equal` keeps the signs;
  - a bad overlap, or a step of 0, is refused.
- **Grids:**
  - the raster's and the snake's orders, exactly;
  - the C-Chip pattern's 35 sites;
  - counts or steps of 0 are refused;
  - **over 300 random grids,** each in raster and snake: every site once, all within the grid, and each snake move to a neighbour;
  - **the z:** the focused site fewest moves away, the latest when two tie.
- **The spiral:**
  - **a round sample:** every grid site on it is imaged; the first nine visits are the 3 × 3 around the start; the gap at its edge in x is filled at 550; it finishes `SurroundedByBackground`;
  - **its range:** with sample everywhere, it visits exactly the grid sites within range, and finishes `ReachedRange`;
  - **its properties, over 60 random samples of 1 to 4 disks,** with random steps and ranges:
    - each main site once, on the grid, within range, and next to an image already found;
    - each between site halfway between an image and background;
    - every gap between an image and background beside it filled;
  - **the z:**
    - worked by hand: the lowest focused site near by, and unfocused background not counting;
    - **over 40 random scans,** with autofocus failing two times in three, the z always follows the spec's rule, including sites with no focus near by.

**Mutation checks:** I broke each of 13 behaviours. A test failed for 12 of them:

- **the step:**
  - the axes swapped;
  - `equal` losing a sign;
- **grids:**
  - the snake not turning;
  - the grid's z from the earliest tie;
- **the spiral:**
  - no range;
  - the range not reported;
  - no gap filled from an image, or from background;
  - the start not weighed in the order;
  - between sites adding sites of their own;
- **the spiral's z:**
  - never "near";
  - the highest z.

The 13th, between sites first, makes no difference to the order, as above.

At first, two of those 12 survived, and a third was caught only by a hang. The tests grew to catch them:

- `equal`'s sign, which a positive `dx` hid;
- the gaps filled from an image, now checked over the random samples;
- a spiral that never ends, now stopped after 5,000 visits rather than hanging.

## Follow-ups and known gaps

- **The scan engine** (P35) drives these planners, and gives the regular grids their workflows. The spiral and the C-Chip pattern get theirs in P37.
- **A plan isn't saved.** A scan's folder will record its settings and visits (P35).
- **The step assumes no rotation or shear** beyond a quarter turn: each axis takes only the stage's motion along it, as OpenFlexure's does.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
