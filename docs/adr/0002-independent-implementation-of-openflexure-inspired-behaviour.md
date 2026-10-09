# ADR-0002: Independent implementation of OpenFlexure-inspired behaviour

- Status: Accepted
- Date: 2026-10-08
- Phase: P00

## Context

The [goals](../milestone-1/goals.md) ask for as much feature parity with the OpenFlexure Microscope as possible, an original Vue/TypeScript interface inspired by OpenFlexure, and an image simulator inspired by OpenFlexure's. Planning studied the OpenFlexure Microscope Server (v3 branch) closely, including its web app and its Things.

The OpenFlexure Microscope Server, web app included, is licensed under **GPL-3.0**. This repository is **MIT**, as is `teta-wot`. Copying or translating GPL code, styles or assets into this repository would make the combined work subject to the GPL, which conflicts with the project's licence.

## Decision

OpenFlexure is a **behavioural reference**, never a source.

**Allowed.** Using what OpenFlexure *does*:

- its feature set, workflows and interaction patterns (for example the tab structure, the calibration wizard flow, double-click to move);
- layout proportions and the overall visual language (a vertical rail, a prominent live image, a magenta accent), expressed through our own components and theme tokens;
- Thing and affordance names (`camera`, `stage`, `fast_autofocus`, `sample_scan`, …), which we mirror on purpose for familiarity (ADR-0007, P02), and default values such as port 5000 or an overlap of 0.35;
- concepts and algorithms at the level of a description (for example "sweep z while monitoring JPEG size and return to the sharpest point").

**Not allowed:**

- copying or translating code, comments or docstrings, or porting a file line by line, whatever the language;
- copying styles (LESS/CSS), icons, logos, images, sample sprites or test fixtures;
- copying UI copy, help text or documentation prose. Our wording is our own.

**Process:**

- **Spec first.** Phases that implement OpenFlexure algorithms (P21, P23, P27, P30, P34, P36, P37, P41) open their implementation notes with a behaviour spec, written in our own words before any code. The implementation is written from that spec and its tests, not with OpenFlexure's source open side by side.
- **Reference screenshots stay out of git.** Captures of OpenFlexure's UI (P05) live in git-ignored folders and are used only for side-by-side review.
- **Dependency licences** must allow MIT distribution: MIT, Apache-2.0, BSD, ISC or Zlib. MPL-2.0 is acceptable only as an unmodified development dependency.
- **Every pull request** ticks the independent-implementation box in the template, and every notes file repeats it.
- **When in doubt,** leave it out, and raise it in the pull request.

This is an engineering policy, not legal advice.

## Consequences

- The project can stay MIT, and downstream users aren't bound by the GPL.
- Algorithm phases take more effort, since behaviour has to be specified and tested rather than ported. The specs and tests become useful documentation.
- Some behaviour will differ in detail from OpenFlexure. The phase notes record the differences that matter.
- Mirrored names make the API familiar to OpenFlexure users without promising compatibility.

## Alternatives considered

- **License this project under GPL-3.0 and reuse OpenFlexure's code.** It would be faster in places, but the owner chose MIT, consistent with `teta-wot`, and the GPL would bind every downstream user and distributor.
- **A strict clean room, where implementers never read the reference.** It gives the strongest separation, but it's impractical for a one-person project whose planning has already studied the reference. The spec-first process gives most of the benefit.
- **No policy.** It risks accidental copying through habit or convenience, and leaves no record of how the code was written.
