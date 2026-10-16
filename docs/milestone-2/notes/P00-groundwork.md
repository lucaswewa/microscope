# P00: Milestone 2 groundwork

- Status: Done
- Pull request: [#71](https://github.com/lucaswewa/microscope/pull/71)
- ADRs: [ADR-0038](../../adr/0038-milestone-2-platforms.md), superseding ADR-0003
- Spec: [phases.md#p00](../phases.md#p00)

## Summary

Milestone 2's folders, rules and platform decision, before its first code.

- **ADR-0038:** milestone 2's platforms are Windows x86_64, Linux x86_64 and Linux ARM64 on a Raspberry Pi 5, with CI on all three. It supersedes ADR-0003, milestone 1's Windows-only platform.
- **These notes' folder,** with an index and a template. The template has a **Hardware** section: what each phase was tried on, and the owner's run.
- **`CLAUDE.md`** for milestone 2:
  - the plan and the branch names;
  - the platforms;
  - the hardware rules: environment variables, fakes, SDKs never redistributed, protocols in our own words;
  - a driver phase is Done only after the owner's run on the device.
- **[`hardware.md`](../hardware.md):** the owner's devices, the SDKs found on the owner's PC, and the hardware tests' variables.

## What was built

| Where | What |
|---|---|
| `docs/adr/0038-milestone-2-platforms.md` | The platforms |
| `docs/adr/0003-…`, `docs/adr/README.md` | ADR-0003 marked superseded; ADR-0038's row |
| `docs/milestone-2/notes/` | The index, the template, and these notes |
| `docs/milestone-2/hardware.md` | The devices, the SDKs and the hardware tests' variables |
| `CLAUDE.md` | Milestone 2's plan, platforms and hardware rules |
| `docs/milestone-2/implementation-plan.md` | Approved; P00 In review; the review points' outcome in the change log |
| `docs/README.md` | Milestone 2's hardware and notes |

## How to try it

Read ADR-0038, `hardware.md` and the new rules in `CLAUDE.md`. There's no code.

## Hardware

**On the owner's Windows PC,** on 2026-10-16:

| What | Found |
|---|---|
| Basler pylon | 26.09.2, with its C API (`PylonC_v10.dll`, 64-bit) and its camera emulator |
| Allied Vision Vimba X | 2026-3, with its C API (`VmbC.dll`) |
| Zaber Motion Library | 10.2.0, though D3 drives Zaber stages through their protocol instead |
| Devices | None connected at the time |

**The devices' models** aren't recorded yet. `hardware.md` has a row for each, for the owner to fill in, or to tell me.

## Design notes and deviations from the plan

- **The review points** in [plan §10](../implementation-plan.md#10-points-for-your-review) stand as proposed, with the plan's approval:
  - the SDKs are loaded at run time, to be confirmed in P03;
  - hardware tests are run by the owner;
  - P09 tries both ways of reaching the Pi camera;
  - tokens go over the trusted network in milestone 2;
  - the Sangaboard's protocol is written from its documentation;
  - desktop updates are out.
- **Two points are still open:**
  - whether `teta-wot` may change for Linux, which P01 asks first;
  - the hardware's models.
- **Both SDKs' C APIs are on the owner's PC,** which is what loading them at run time needs on Windows. P03 checks Linux and ARM64.
- **pylon's camera emulator** could let the Basler driver's tests run on a PC with pylon but no camera.
- **The hardware tests' variables** in `hardware.md` are planned names; each driver phase settles its own.
- **Size:** docs only.

## Tests

- None: docs only. CI runs as usual.

## Follow-ups and known gaps

- **The hardware's models,** in `hardware.md`.
- **P01** starts with the `teta-wot` question.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
