# Settings -- Feature-Level Contract

---

## Persistence

1. Settings SHALL persist to `~/Library/Application Support/rick-command-center/settings.json` via `SettingsService`.
2. Writes SHALL be atomic (write temp + rename).
3. Reads SHALL fall back to `DEFAULT_SETTINGS` for any missing field.
4. Settings SHALL be loaded once at app startup and pushed to renderer via `IPC.GetSettings` on demand.

## Modal Open / Close

5. The modal SHALL be opened by clicking the gear icon in the top bar.
6. Cancel SHALL close the modal WITHOUT writing.
7. Save SHALL write the patch and close on success.
8. Failed saves SHALL keep the modal open and show no toast in v1 (errors are silent).

## Field Validation

9. `recentSessionDays` SHALL clamp to a minimum of 1 (zero or negative becomes 1).
10. `warnThreshold` and `criticalThreshold` SHALL be stored as fractions (0..1) but rendered as integer percents (10..100).
11. `customTerminalCommand` SHALL be trimmed before save; empty string SHALL be saved as `undefined`.
12. `branchPrefix` and `defaultBranchOff` SHALL accept any string (no slug validation in v1).

## Terminal App Choice

13. The dropdown SHALL show all `TerminalApp` enum values.
14. The "Custom terminal command" field SHALL be visible ONLY when `terminalApp === 'custom'`.
15. When `terminalApp === 'custom'` is saved with no command, the launch flow SHALL fail with a descriptive error (not silently skip).

## Skip Permissions

16. The checkbox SHALL default to OFF.
17. When ON, the launcher SHALL append `--dangerously-skip-permissions` to every `claude` invocation.
18. The flag SHALL apply globally -- there is no per-workflow override in v1.

## Patch Merge

19. `setSettings(patch)` SHALL deep-merge the patch into current settings:
    - Top-level fields are overwritten by the patch.
    - `panelSizes` is shallow-merged with current (preserves keys not in patch).
    - `customTitles` is shallow-merged.
    - `archivedSessionIds` is replaced wholesale by the patch when present.
20. `setSettings` SHALL return the full new `AppSettings` for the renderer to consume.

## Hidden Field Updates

21. `lastUniverse` SHALL be updated by the universe dropdown in the top bar -- not via this modal.
22. `panelSizes` SHALL be updated by drag interactions on resize handles.
23. `archivedSessionIds` SHALL be updated by session-card discard (`×`).
24. `customTitles` SHALL be updated by session-card rename interactions.
25. `pluginInstalled` SHALL be updated only by the plugin install consent flow.

## Default Settings

26. The initial values SHALL be:
    - `warnThreshold: 0.70`
    - `criticalThreshold: 0.90`
    - `recentSessionDays: 7`
    - `archivedSessionIds: []`
    - `branchPrefix: "feature/"`
    - `defaultBranchOff: "dev"`
    - `terminalApp: "Terminal"`
    - `skipPermissions: false`
27. All optional fields SHALL be unset on first run.

## IPC Surface

28. `IPC.GetSettings` SHALL return the full `AppSettings`.
29. `IPC.SetSettings` SHALL accept a `Partial<AppSettings>` patch and return the merged result.
30. `IPC.SettingsUpdate` is NOT used in v1 -- the renderer pulls via `setSettings` rather than subscribing to push.

---

## Open Questions

- OQ1: Should crit threshold enforce `>= warn`?
- OQ2: Should the modal expose `archivedSessionIds` as a "restore" UI?
- OQ3: Should there be a "Reset to defaults" button?
- OQ4: Should per-workflow settings overrides exist (e.g. terminalApp per workflow)?
