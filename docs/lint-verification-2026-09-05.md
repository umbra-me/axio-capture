# Frontend lint verification — 2026-09-05

The standalone package now exposes `lint` (ESLint over all `src` TypeScript/TSX)
and `typecheck` (`tsc --noEmit`) as separate gates. Generated output is excluded;
source files and rules are not suppressed to make the gate pass.

Run `pnpm install --frozen-lockfile`, then
`pnpm run lint`, `run typecheck`, `test`, and `run build` with the same package manager.
All four commands passed on macOS on September 5: 4 existing frontend tests,
zero lint errors or warnings, successful TypeScript and production frontend build.
A temporary `src/audit-invalid-lint.ts` containing an invalid declaration was
rejected with exit 1 and a parsing error, then removed in a finally block.
The aggregate rejection receipt is `/tmp/axio-lint-rejection-20260905.json` on the
audit workstation.

ESLint 10 JavaScript/TypeScript recommended rules are enabled. The undo/redo
shortcut uses an explicit if/else with unchanged Shift-Z/Z behavior. No Rust,
release version, signing, updater configuration, or native acceptance status was
changed. Existing native signing and interaction gates remain in the change-set
receipt.
