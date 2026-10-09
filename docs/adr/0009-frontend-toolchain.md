# ADR-0009: Frontend toolchain

- Status: Accepted
- Date: 2026-10-09
- Phase: P04

## Context

- The [goals](../milestone-1/goals.md) fix the core: Vue 3 with TypeScript, Pinia, Vite and Vue Router. The web app must also run in a Tauri desktop app later (ADR-0010). CI runs on Windows only (ADR-0003).
- As of October 2026, the current majors are Vite 8, Vue 3.5, Vue Router 5, Pinia 4, Vitest 5 and ESLint 10. TypeScript 7, the native compiler, is out, but `typescript-eslint` (8.71) supports only TypeScript below 6.1.
- npm has had several supply-chain incidents where compromised versions of popular packages were published and then caught within days.

## Decision

**Project.** The app lives in `web/`, an npm project outside the Cargo workspace. It uses Node 24 LTS (`web/.nvmrc`, `engines`) and npm, with `package-lock.json` committed and `npm ci` in CI.

**Stack:**

- Vite 8 for development and builds.
- Vue 3.5, with single-file components using `<script setup lang="ts">`.
- TypeScript 6.0, strict through `@vue/tsconfig`, pinned to `~6.0.3` until `typescript-eslint` supports 7.
- `vue-tsc` 3, which type-checks `.vue` files across three project references: the app, the Node-side config files and e2e tests, and the unit tests.
- Pinia 4, and Vue Router 5 with hash history (ADR-0010).

**Quality:**

- ESLint 10 with a flat config: `eslint-plugin-vue`'s recommended rules, `@vue/eslint-config-typescript`'s recommended rules, and the Tauri-readiness rules (ADR-0010). Zero warnings are allowed.
- Prettier 3 formats (no semicolons, single quotes, 100 columns), and ESLint's formatting rules are off.
- CI checks formatting, lint, types, unit tests and the build.

**Tests:**

- Vitest 5 with happy-dom and `@vue/test-utils`, for unit and component tests in `tests/unit/`.
- Playwright with Chromium, for end-to-end tests in `tests/e2e/` against the production build served by `vite preview`. It runs locally for now. The `e2e` CI job comes with P12, when the server serves the app.

**Layout:** `src/` holds `main.ts`, `App.vue`, `router/`, `views/` and `host/`. Later phases add the folders in [plan §5.10](../milestone-1/implementation-plan.md#510-web-app). `@` resolves to `src/`.

**Scripts:** `dev`, `build`, `preview`, `typecheck`, `lint`, `lint:fix`, `format`, `format:check`, `test:unit`, `test:unit:watch` and `test:e2e`.

**Development server:** `/api` (WebSockets included) is forwarded to `http://127.0.0.1:5000`, or to `MICROSCOPE_API_TARGET` if that's set.

**Version:** `package.json`'s version tracks the Rust workspace's (0.1.0). The app reads it as `__APP_VERSION__`.

**Dependency age:** new dependencies and updates are installed with `npm install --before <date 14 days ago>`. Nothing published in the last two weeks gets into the lockfile, and the ranges in `package.json` start at the versions resolved.

## Consequences

- Types, lint, format and tests are checked on every pull request. Each runs in seconds once dependencies are cached.
- TypeScript 7's faster compiler waits for `typescript-eslint`. Moving to it is a deliberate upgrade later.
- Adding or updating a dependency needs the `--before` date. Updates for urgent security fixes may justify a shorter delay; such updates are noted in the pull request.
- Playwright downloads its browsers once per machine (about 150 MB, into `%LOCALAPPDATA%\ms-playwright`).

## Alternatives considered

- **pnpm.** It's faster and stricter, and it has a built-in minimum release age, but it's one more tool to install. npm comes with Node.
- **TypeScript 7 now.** `typescript-eslint` doesn't support it yet.
- **jsdom instead of happy-dom.** jsdom is more complete but slower. A test that needs it can switch with Vitest's per-file environment comment.
- **Jest.** It's slower, and its ESM and Vite integration is awkward next to Vitest.
- **Biome instead of ESLint and Prettier.** It's faster, but it doesn't lint Vue templates as `eslint-plugin-vue` does.
- **The `create-vue` generator.** It produces the same structure plus boilerplate to delete. Writing the files by hand keeps each one reviewable.
