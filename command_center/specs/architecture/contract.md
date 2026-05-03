# Architecture -- Invariants

System-level rules and non-functional requirements. Numbered for cross-referencing from PRD § 7.

---

## Read-Only Discipline

1. The app SHALL NOT write to any path under `~/.rick/state/` or `~/.rick/universes/` (NFR-5). A runtime guard SHOULD refuse such writes (not enforced as code in v1; convention).
2. The plugin hooks SHALL be the only writers to `~/.rick/tracking/<sid>.md` -- the app process never writes to this directory.
3. The app's only write to `~/.claude/settings.json` SHALL happen through the install consent flow.
4. The app SHALL back up `settings.json` before any modification (`settings.json.rcc.bak.<ts>`).

## Process Boundary

5. Main, preload, and renderer SHALL communicate exclusively via IPC. No globals are shared across the contextBridge.
6. The renderer SHALL NOT have Node access (`nodeIntegration: false`).
7. All filesystem reads/writes SHALL happen in main.
8. PTY spawning SHALL happen in main (`PtyService`); renderer issues commands via IPC.

## File Watcher Behavior

9. Transcript watcher SHALL be `ignoreInitial: true`; initial scan done explicitly by `initialScan()`.
10. Transcript watcher SHALL fire on add + change events for any `.jsonl` under `~/.claude/projects/**` at depth ≤ 2.
11. Tracking watcher SHALL fire on add + change + unlink for any `.md` under `~/.rick/tracking/` at depth 1.
12. Universe watcher SHALL fire on add + change + unlink + addDir + unlinkDir under `~/.rick/universes/` at depth ≤ 4, debounced at 250ms before rescanning.
13. All watchers SHALL ignore paths containing `/.git/` and paths ending in `.DS_Store`.

## Composition / Re-compute

14. `SessionsService.recompute()` SHALL run on transcript update, tracking update, and tracking removal.
15. `recompute()` SHALL emit the new full `Session[]` to the renderer over `IPC.SessionsUpdate`.
16. `recompute()` SHALL apply: cutoff filter (`recentSessionDays`), universe filter (`lastUniverse`), archived filter (`archivedSessionIds`).
17. `recompute()` SHALL run `propagateContinuations` BEFORE archived filtering.
18. `recompute()` SHALL sort the result by `lastActivity` descending.

## Correlation On-Demand

19. `correlateWorkflow` SHALL re-parse the entire JSONL on every call.
20. `correlateWorkflow` SHALL return `null` when the transcript file cannot be opened.
21. `correlateWorkflow` SHALL be invoked via `listSessionFiles(sid)` only -- not on transcript updates.
22. The renderer SHALL trigger re-fetch by invoking `listSessionFiles` on selection AND on `lastActivity` ticks (see [`../workflow-status/contract.md`](../workflow-status/contract.md) §44).

## IPC Channels

23. Push channels SHALL be: `UniversesUpdate`, `WorkflowsUpdate`, `SessionsUpdate`, `PtyData`, `PtyListUpdate`, `PtyExit`.
24. Push channels SHALL NEVER be invoked from renderer to main; they are main → renderer only.
25. Request/response channels SHALL use `ipcRenderer.invoke` / `ipcMain.handle` -- never `send`/`on` for getters.

## Performance Targets (NFR-1, NFR-2, NFR-3, NFR-4)

26. NFR-1: Filesystem-change → UI-update p95 SHALL be ≤ 1s for tracking and JSONL files up to 50 MB.
27. NFR-2: The app SHALL be stable with ≥ 10 concurrent sessions and ≥ 10 watched JSONL files without UI jank.
28. NFR-3: Steady-state RSS SHALL be ≤ 600 MB with 10 sessions open.
29. NFR-4: Cold start to first usable UI SHALL be ≤ 3s on an M-series Mac.

## Crash Resilience (NFR-6)

30. A malformed YAML, JSONL, or tracking file SHALL be reported (via toast in v2; silently skipped in v1) and SHALL NOT take down the app or sibling sessions.
31. Hook failures SHALL surface to the user via Claude Code's hook-error UI; the app continues to render whatever state remains.
32. PTY child crashes SHALL flip the `alive` flag to false; the tab persists with a rose dot.

## Schema Resilience (NFR-9)

33. Tracking frontmatter parsing SHALL be permissive: bad lines skipped, unknown keys preserved on round-trip.
34. Workflow YAML parsing SHALL handle both shorthand (`param: default`) and structural (`param: {type, default, ...}`) forms.
35. Unknown model ids in JSONL SHALL fall back to a 200k context limit and an unknown-model `?` indicator.

## Privacy (NFR-8)

36. The app SHALL make NO network calls beyond what `claude` itself makes inside the PTY.
37. The app SHALL NOT collect telemetry.

## Single-Window Lock (Distribution)

38. The packaged app SHALL use `app.requestSingleInstanceLock()` so launching from /Applications focuses the existing window instead of stacking.
39. The dev workflow (`npm run dev`) does NOT use the lock; predev cleanup handles stacking instead.

## Build & Distribution (NFR-7)

40. `npm run package:dmg` SHALL produce a `.dmg` at `release/Rick Command Center-<version>-<arch>.dmg`.
41. The `.dmg` SHALL be ad-hoc-signed (`identity: null` in `electron-builder` mac config). `hardenedRuntime` SHALL be off (incompatible with ad-hoc).
42. The `.dmg` SHALL ship without notarization in v1 -- recipient does Right-click → Open the first launch.
43. `node-pty` native binaries SHALL be rebuilt as part of `electron-builder` packaging (`@electron/rebuild`).

---

## Open Questions

- OQ1: Should `correlateWorkflow` be memoized per session/lastActivity to avoid the re-parse on every `listSessionFiles` call?
- OQ2: Should a single multi-platform build target be added (universal arm64+x64 dmg)? Currently arm64-only.
- OQ3: Is single-instance lock the right choice, or should the app support per-window-per-universe layouts?
- OQ4: Should there be a "headless" diagnostic mode that runs the watchers and emits stats without a UI window?
