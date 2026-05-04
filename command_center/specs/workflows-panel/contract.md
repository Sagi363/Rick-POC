# Workflows Panel -- Feature-Level Contract

---

## Discovery

1. The app SHALL scan `~/.rick/universes/<u>/workflows/*.{yaml,yml}` for every universe it discovers.
2. Universes are discovered as direct subdirectories of `~/.rick/universes/` whose name does not start with `.`.
3. The watcher SHALL fire on add, change, unlink, addDir, unlinkDir events.
4. Watcher updates SHALL be debounced at 250ms before rescanning -- multiple rapid file events coalesce into one push.
5. Files inside `.git/` SHALL be ignored.
6. Files named `.DS_Store` SHALL be ignored.
7. Watcher depth SHALL be limited to 4 (matches `<universes-root>/<u>/<workflows>/<file>`).

## Workflow Parsing

8. The YAML body SHALL be parsed via `js-yaml`.
9. When parsing fails (malformed YAML), the file SHALL be skipped silently in v1 -- it is not surfaced in the UI.
10. The workflow `name` SHALL be the YAML `name:` field; when absent, derived from the filename minus the `.yaml`/`.yml` extension.
11. The workflow `description` SHALL be the YAML `description:` field if present; otherwise null.
12. Each step's `agent` SHALL be `step.agent` if present; else `step.uses` if present; else the literal `"unknown"`.
13. The workflow `agents` array SHALL be the unique set of step agents, excluding `"unknown"`.
14. The workflow `dependsOn` array SHALL be the unique flattened list of all `step.depends_on` values.

## Param Normalization

15. When a param value under `params:` is an object, it SHALL be parsed structurally: `type`, `default`, `description`, `required`, `enum`.
16. When a param value under `params:` is a primitive (string, number, boolean, null), it SHALL be treated as the default value.
17. When `type:` is missing, the type SHALL be inferred:
    - `boolean` → `"bool"`
    - `number` (integer-valued) → `"int"`
    - `string` → `"string"`
    - anything else → `"unknown"`
18. A param SHALL be marked `required: true` only when the YAML explicitly says `required: true`.
19. `enum:` SHALL trigger select-style rendering in the launch modal; non-list `enum:` SHALL be ignored.

## Filtering

20. The list of workflow cards SHALL be filtered to those whose `universe === settings.lastUniverse`.
21. Workflows whose universe is unbound (no `lastUniverse` set) SHALL all show.
22. Search SHALL be case-insensitive substring match against `name`, `description`, and any agent name in `agents`.

## Empty States

23. When no universes are discovered, the workflow tab SHALL display "Pick a universe in the top bar."
24. When a universe is selected but has zero workflows, the panel SHALL display the path to drop a YAML into.

## Click → Launch

25. Clicking a workflow card SHALL fire `onLaunchWorkflow(workflow)` (the Drawer prop).
26. The Launch Modal SHALL open pre-bound to the chosen workflow (see [`../launch-modal/spec.md`](../launch-modal/spec.md)).
27. The card SHALL NOT show a YAML preview -- preview happens after a session launches and the YAML is reachable from the file list.

---

## Open Questions

- OQ1: Should malformed YAML surface as a toast pointing at the file, or stay silent?
- OQ2: Should workflows be sorted alphabetically vs by file mtime?
- OQ3: Do we want a "favorite" mark for workflows the user runs often?
