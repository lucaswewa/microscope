I want to build a microscope service built with `teta-wot` (..\teta-wot), with an original Vue/TypeScript interface inspired by OpenFlexure Microscope (..\openflexure-microscope-server). For the first milestone, I want to build as many features/functionalities parity with the OpenFlexure Microscope with a simulated camera and XYZ stage in the first milestone. Once we are confident with the simulated camera, we start building real-camera and real-stage support in the second milestone.

I want to make an implementation plan for the first milestone first.

In this milestone, I want to:
- Build a Rust backend using the `teta-wot` package, with a OpenFlexure Microscope inspired web application.
- Develop against a simulated microscope and a simulated stage.
- Reuse the web application in a later Tauri desktop application
- Support a managed local microscope service and a remote microscope connection
- The main UI and workflow can be similar to the OpenFlexure Microscope's.
  + Use a vertical navigation rail with View, Control, Slide Scan, Sequence, and Gallery at the top; and Settings, Logging, About, and Power below.
  + Favor the reference's compact controls, prominent live image, magenta accent, light neutral surfaces, and dark charcoal surfaces.
  + Reproduce the workflows and proportions through original components and theme tokens.
  + Visual fidelity will be reviewed against reference screenshots in both themes.
- Frontend stack: Vue3 and TypeScipt, with Pinia, Vite, and Vue Router
- Use the image simulator for stage-dependent preview, focus, and scan tiles. The image simulator can be inspired by the OpenFlexure Microscope's.
- Dark, Light and Follow System themes; persist the user's choice

I want to implement this app in a progressive phase way. Each implementation phase should be small enough for me to review the code, but should be self-contained for a defined task/job.

All the ADRs and implementation notes should be created for each implementation phase. And they should be saved to the /docs folder.

I want to create the implementation plan for the first milestone. Ask questions as much as needed. Then save the plan to the /docs folder for me to review. Don't make any code change yet.
