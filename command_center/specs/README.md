# Rick Command Center -- Specs Index

Per-feature specifications. See [`PRD.md`](../PRD.md) for the requirements view; this folder is the implementation contract.

Format follows the `ACC_Rewrite_universe` convention: each feature gets its own folder with a `spec.md` (presentation/behavior) and a `contract.md` (numbered business rules). Multi-component features add a `screen-composition.md`.

## Cross-cutting

| Feature | Spec | Contract |
|---|---|---|
| System architecture (Claude ↔ Rick ↔ CC) | [`architecture/spec.md`](architecture/spec.md) | [`architecture/contract.md`](architecture/contract.md) |
| Plugin hooks (UserPromptSubmit, PostToolUse, Notification, Stop) | [`hooks/spec.md`](hooks/spec.md) | [`hooks/contract.md`](hooks/contract.md) |
| Rick ↔ correlator protocol | [`rick-contract/spec.md`](rick-contract/spec.md) | [`rick-contract/contract.md`](rick-contract/contract.md) |

## Surfaces (UI)

| Feature | Spec | Contract | Composition |
|---|---|---|---|
| Sessions panel (drawer Sessions tab + cards) | [`sessions-panel/spec.md`](sessions-panel/spec.md) | [`sessions-panel/contract.md`](sessions-panel/contract.md) | [`sessions-panel/screen-composition.md`](sessions-panel/screen-composition.md) |
| Workflows panel (drawer Workflows tab) | [`workflows-panel/spec.md`](workflows-panel/spec.md) | [`workflows-panel/contract.md`](workflows-panel/contract.md) | -- |
| Launch modal (new workflow + worktree dialog) | [`launch-modal/spec.md`](launch-modal/spec.md) | [`launch-modal/contract.md`](launch-modal/contract.md) | -- |
| Workflow status panel (phase rings) | [`workflow-status/spec.md`](workflow-status/spec.md) | [`workflow-status/contract.md`](workflow-status/contract.md) | -- |
| Progress / activity panel | [`progress-panel/spec.md`](progress-panel/spec.md) | [`progress-panel/contract.md`](progress-panel/contract.md) | -- |
| Terminal (xterm + node-pty + quick commands) | [`terminal/spec.md`](terminal/spec.md) | [`terminal/contract.md`](terminal/contract.md) | [`terminal/screen-composition.md`](terminal/screen-composition.md) |
| Settings window | [`settings/spec.md`](settings/spec.md) | [`settings/contract.md`](settings/contract.md) | -- |

## Format conventions

Each spec follows this structure (mirroring the `ACC_Rewrite_universe` template):

**`spec.md`** -- the "what":
1. **Overview** -- one paragraph; what this surface or system does and why.
2. **Source files:** -- list of code anchors with line numbers at the top.
3. **Presentation Models** -- numbered subsections (2.1, 2.2 …) with field tables (Field / Type / Optional / Example / Notes) for every domain object the surface deals with.
4. **Visual States** -- tables of layouts, sub-states, tone branches.
5. **Interactions** -- target / gesture / result table.
6. Feature-specific sections (Search, Pinning, Live Refresh, etc.).
7. **Cross-References** + **Source Files** + **Open Questions**.

**`contract.md`** -- the "rules":
- Numbered, normative ("SHALL" / "SHOULD") rules, ordered by topic and grouped under topic headings.
- Cross-referenceable from `spec.md` via § N.
- Trailing **Open Questions** (OQ1, OQ2 …).

**`screen-composition.md`** -- only when the feature has ≥ 2 named components:
- Component tree (ASCII).
- Ownership map table (component / view file / owner / data source).
- Layout zones + flow diagram.
- Cross-references + source files.

## Title style

Headings follow the rewrite-universe convention: `# <Feature> -- <Type>` with a double-dash separator.

## Keeping these in sync

When you add or rename a surface in code, update the matching `spec.md` and `contract.md`. When you change a regex in `correlation.ts` or a hook in `plugin/hooks/`, update `rick-contract/contract.md` AND `~/.rick/persona/rules.md` simultaneously. Drift between this folder and code is the #1 maintenance hazard -- `PROGRESS.md` calls out drifts as they're discovered.
