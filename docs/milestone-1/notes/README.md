# Milestone 1 implementation notes

One file per phase, written in that phase's pull request from [template.md](template.md). The notes say what was built, how to try it, how it was designed, where it departs from the [phase spec](../phases.md), and what's left to do. The reasons behind significant decisions go in [ADRs](../../adr/README.md).

## Status

A phase's notes are marked *In review* in its own pull request. The next phase's pull request marks them, and the phase's row in the [plan's overview](../implementation-plan.md#8-phase-overview), *Done*, since a merged pull request can't be edited.

## Index

| Phase | Notes | Status |
|---|---|---|
| P00 | [Docs scaffolding and foundational ADRs](P00-docs-scaffolding.md) | Done |
| P01 | [Rust workspace and CI](P01-rust-workspace.md) | Done |
| P02 | [Server lifecycle and composed router](P02-server-lifecycle.md) | Done |
| P03 | [Server logging and log endpoints](P03-server-logging.md) | Done |
| P04 | [Web app scaffold and tooling](P04-web-scaffold.md) | Done |
| P05 | [OpenFlexure reference screenshots](P05-reference-screenshots.md) | Done |
| P06 | [Design tokens and theme switching](P06-design-tokens.md) | Done |
| P07 | [App shell and navigation rail](P07-app-shell.md) | Done |
| P08 | [UI components I: controls](P08-ui-controls.md) | Done |
| P09 | [UI components II: overlays and feedback](P09-overlays.md) | Done |
| P09b | [UI components III: menus, lists and shortcuts](P09b-menus-shortcuts.md) | Done |
| P10 | [WoT client library](P10-wot-client.md) | Done |
| P11 | [Connection management](P11-connection.md) | Done |
| P12 | [Serve the web app from the backend](P12-serve-webapp.md) | Done |
| P13 | [Simulation world, optics and blob specimen](P13-simulator-core.md) | Done |
| P14 | [Simulated stage motion model](P14-stage-model.md) | Done |
| P15 | [Hardware interfaces and units](P15-hardware-interfaces.md) | Done |
| P16 | [Simulated stage Thing](P16-simulated-stage.md) | Done |
| P17 | [Simulated camera and illumination Things](P17-simulated-camera.md) | Done |
| P17b | [Capture buffer and settling](P17b-capture-buffer.md) | Done |
| P18 | [MJPEG streaming in the client](P18-mjpeg-streaming.md) | Done |
| P18b | [View tab: live image](P18b-view-tab.md) | Done |
| P19 | [Generated API types and typed facades](P19-api-types.md) | Done |
| P20 | [Control tab: stage navigation](P20-control-tab.md) | Done |
| P20b | [Control tab: wheel focus and navigation preferences](P20b-wheel-and-preferences.md) | Done |
| P21 | [Autofocus](P21-autofocus.md) | Done |
| P22 | [Image registration core](P22-image-registration.md) | Done |
| P23 | [Camera–stage mapping](P23-camera-stage-mapping.md) | Done |
| P23b | [Click-to-move](P23b-click-to-move.md) | Done |
| P24 | [Server-described UI elements](P24-server-ui.md) | Done |
| P24b | [Server-described action and download buttons](P24b-server-buttons.md) | Done |
| P25 | [Specimens II and optical effects](P25-specimens-and-optics.md) | Done |
| P26 | [Capture to the data folder](P26-capture.md) | Done |
| P27 | [Background detection and camera calibration](P27-background-calibration.md) | Done |
| P28 | [Settings tab](P28-settings-tab.md) | Done |
| P29 | [Calibration wizard](P29-calibration-wizard.md) | Done |
| P30 | [Stage measurement tools](P30-stage-measure.md) | Done |
| P31 | [Gallery backend and data route](P31-gallery-backend.md) | Done |
| P32 | [Gallery tab](P32-gallery-tab.md) | Done |
| P33 | [Image and deep-zoom viewer](P33-viewer.md) | Done |
| P34 | [Scan planners](P34-scan-planners.md) | Done |
| P35 | [Scan engine with snake and raster workflows](P35-scan-engine.md) | Done |
| P35b | [Scans in the gallery](P35b-scans-in-gallery.md) | Done |
| P36 | [Smart z-stacks](P36-smart-stacks.md) | Done |
| P37 | [Histo smart-spiral and C-Chip workflows](P37-histo-and-c-chip.md) | Done |
| P38 | [Mosaic and deep-zoom pyramid core](P38-mosaic-core.md) | Done |
| P39 | [Live and final stitching](P39-stitching.md) | Done |
| P40 | [Slide Scan tab](P40-slide-scan.md) | Done |
| P41 | [Sequence engine (MDA-ready)](P41-sequence-engine.md) | Done |
| P42 | [Sequence tab](P42-sequence-tab.md) | Done |
| P43 | [Logging tab](P43-logging-tab.md) | Done |
| P44 | [Sidecar process contract](P44-sidecar-contract.md) | Done |
| P45 | [Local launcher](P45-local-launcher.md) | Done |
| P46 | [About and Power tabs](P46-about-and-power.md) | Done |
| P47 | [Visual fidelity and accessibility pass](P47-visual-and-accessibility.md) | Done |
| P48 | [End-to-end, soak and performance](P48-end-to-end-soak-performance.md) | In review |
