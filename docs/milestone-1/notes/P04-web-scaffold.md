# P04: Web app scaffold and tooling

- Status: Done
- Pull request: [#5](https://github.com/lucaswewa/microscope/pull/5)
- ADRs: [ADR-0009](../../adr/0009-frontend-toolchain.md), [ADR-0010](../../adr/0010-rules-for-a-tauri-ready-web-app.md)
- Spec: [phases.md#p04](../phases.md#p04)

## Summary

The web app now exists as a typed Vue 3 skeleton in `web/`, with formatting, linting, type checking, unit tests, an end-to-end smoke test, a build, and a CI job. It shows a placeholder page. Later phases fill it in, starting with the theme (P06) and the shell (P07). The host adapter that keeps the app ready for Tauri is in place, and ESLint enforces three of the Tauri-readiness rules.

## What was built

| Where | What |
|---|---|
| `web/package.json`, `package-lock.json`, `.nvmrc` | Node 24 LTS, npm; the dependencies of ADR-0009, installed with a 14-day minimum release age |
| `web/vite.config.ts` | The Vue plugin, the `@` alias, `__APP_VERSION__`, and the development proxy from `/api` to `http://127.0.0.1:5000` (or `MICROSCOPE_API_TARGET`) |
| `web/tsconfig*.json`, `env.d.ts` | Project references: the app (`@vue/tsconfig`, strict), the Node-side files and e2e tests (`@tsconfig/node24`), and the unit tests |
| `web/eslint.config.js`, `.prettierrc.json` | ESLint flat config (Vue and TypeScript recommended, the `app/tauri-ready` rules, formatting rules off), and Prettier |
| `web/vitest.config.ts`, `playwright.config.ts` | Vitest with happy-dom, and Playwright with Chromium against `vite preview` |
| `web/src/main.ts`, `App.vue`, `router/`, `views/PlaceholderView.vue` | The app: Pinia, the router (hash history), the browser host, and a placeholder page showing the version |
| `web/src/host/` | The `HostAdapter` interface (`info`, `openLink`, `saveFile`, optional `service`), `hostKey`/`useHost()`, and `createBrowserHost()` |
| `web/tests/unit/`, `web/tests/e2e/` | Unit tests for the browser host and the app; the Playwright smoke test |
| `.github/workflows/ci.yml` | The `web` job on `windows-latest`: `npm ci`, format, lint, type check, unit tests, build |

## How to try it

```powershell
cd web
npm ci
npm run dev        # http://localhost:5173/ shows the placeholder page
npm run lint; npm run typecheck; npm run test:unit; npm run build
npx playwright install chromium   # once per machine
npm run test:e2e
```

With `microscope-server` running on port 5000, <http://localhost:5173/api/v1/health> reaches it through Vite's proxy.

## Design notes and deviations from the plan

- **Versions** (as of 2026-10-09): Vue 3.5.43, Vue Router 5.3.1, Pinia 4.0.3, Vite 8.3.1, Vitest 5.0.1, ESLint 10.11, Prettier 3.9, Playwright 1.63, `vue-tsc` 3.3.11.
  - **TypeScript is 6.0.3, not 7.** `typescript-eslint` supports only `<6.1`.
  - **Every version is at least two weeks old.** They were installed with `npm install --before=2026-09-25`, because several of the latest releases were days old. ADR-0009 makes this the rule for later phases.
- **Tauri-readiness enforcement.** The spec asked only for ADR-0010 to list the rules. ESLint now also rejects `createWebHistory`, `EventSource` and `window.open` outside `src/host/`. A file with all three was checked to fail, then deleted.
- **The `HostAdapter`'s `service`** is optional and absent in the browser. The launcher (P45) manages the local service then, and the Tauri app will provide it later. `ServiceState` and `ServiceControl` are defined now so the shape is reviewed early.
- **Saved files' object URLs** are revoked 10 seconds after the download starts, because revoking at once can cancel downloads in some browsers.
- **Playwright isn't in CI yet,** as the plan has it: the `e2e` job arrives in P12. The smoke test passes locally.
- **`vitest.config.ts` imports `./vite.config.ts` with its extension,** as Vite's coming native config loader requires. `tsconfig.node.json` allows `.ts` extensions in imports for that reason.

## Tests

- **Unit** (5, Vitest):
  - the browser host's info;
  - `openLink` opens a new tab with `noopener,noreferrer`;
  - `saveFile` downloads under the given name, removes its link, and revokes the object URL after 10 seconds, not before;
  - the app shows the placeholder page with the version at `/`;
  - `useHost()` throws a clear error when no host is provided.
- **End to end** (1, Playwright): the production build loads, the title and heading are "Microscope", and the URL is `/#/`.
- **Manual:** with `microscope-server` on port 5091 and `vite` on port 5191 (`MICROSCOPE_API_TARGET=http://127.0.0.1:5091`), the dev server served the app, and `/api/v1/health` and `/api/v1/system/hostname` came back through the proxy.
- **Incident:** an earlier, careless attempt at this check stopped every process listening on port 5000. That included a `microscope-server` the owner was running, started at 04:43 UTC. Manual checks now use their own ports and stop only the processes they started.

## Follow-ups and known gaps

- The e2e CI job (P12), the theme tokens (P06) and the shell (P07).
- The placeholder page has no favicon, so browsers log a 404 for `/favicon.ico`. P07 adds the icon.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
