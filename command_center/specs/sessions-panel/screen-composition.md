# Sessions Panel -- Screen Composition

## Component Tree

```
Drawer (left panel)
  |
  +-- [A] Tab strip (Sessions | Workflows)
  |
  +-- [B] (Sessions tab active)
  |     |
  |     +-- [B1] Search input
  |     +-- [B2] StatusFilterChips (running | waiting | blocked | idle | done)
  |     |
  |     +-- [B3] Sessions list (sorted lastActivity desc)
  |           |
  |           +-- [B3a] Pinned: SessionCard (selected)
  |           +-- [B3b] "— others —" separator (when rest is non-empty)
  |           +-- [B3c] Rest: SessionCard[] (unselected)
  |
  +-- [C] (Workflows tab active -- see workflows-panel/spec.md)
```

### SessionCard subcomposition

```
SessionCard
  |
  +-- [D] Title row
  |     +-- [D1] Title text / inline rename input
  |     +-- [D2] Relative time
  |
  +-- [E] Workflow line (when workflow != null)
  |
  +-- [F] Successor chip (when successorId set)
  |
  +-- [G] Status row
  |     +-- [G1] Status dot + label
  |     +-- [G2] Phase chip (when phase set)
  |     +-- [G3] Step counter (when total set)
  |     +-- [G4] AutoContinuePill (when onSetAutoContinue provided)
  |
  +-- [H] Context bar (when context set)
  |     +-- [H1] Filled progress bar
  |     +-- [H2] used/limit + percent labels
  |
  +-- [I] Hover-only icons (top-right, group-hover:flex)
        +-- [I1] ⤴ Focus terminal (when cwd set)
        +-- [I2] × Discard
```

---

## Ownership Map

| Component ID | View File | Owner | Data Source |
|---|---|---|---|
| Drawer | `src/renderer/src/components/Drawer.tsx` | Renderer (local state for tab/filters/search) | `useAppState().sessions` via IPC push |
| A | `Drawer.tsx` (lines 50-95) | Renderer local state `tab` | -- |
| B1 | `Drawer.tsx` (search input) | Renderer local state `search` | -- |
| B2 | `Drawer.tsx` (chips render) | Renderer local state `enabled` | -- |
| B3 | `Drawer.tsx` (lines 144-192, the IIFE producing pinned + rest) | Derived from props + local state | `sessions` prop |
| B3a, B3c | `SessionCard` | Renderer | `Session` row from props |
| C | -- | Drawer renders WorkflowCard list when `tab === 'workflows'` | `useAppState().workflows` |
| SessionCard | `src/renderer/src/components/SessionCard.tsx` | Renderer (local state for editingTitle, autoContinue) | `Session` prop |
| D1 (rename) | `SessionCard.tsx:124-139` | Local `editingTitle` + `draftTitle` | -- |
| F (successor) | `SessionCard.tsx:168-180` | -- | `session.successorId` |
| G1 (status) | `SessionCard.tsx:181-195` | -- | `session.status` derived in main |
| G4 (auto pill) | `SessionCard.tsx:196-213` | Local `autoContinue` synced to localStorage | `localStorage[rcc:session:auto-continue][id]` |
| H (context bar) | `SessionCard.tsx:197-213` | -- | `session.context` from latest assistant turn |
| I1 (focus) | `SessionCard.tsx:96-107` | Callback to App | `App.tsx` `onFocusTerminal` |
| I2 (discard) | `SessionCard.tsx:108-121` | Callback to App | `App.tsx` `onDiscard` |

---

## Layout Zones

### Zone 1: Drawer container
- **Position:** Left edge of main pane, resizable width via `panelSizes.drawer` setting.
- **Background:** `bg-zinc-950` with right-edge resizer.
- **Layout:** Vertical flex; tab strip top, content below.

### Zone 2: Sessions list
- **Position:** Below tab strip + search + filter chips.
- **Layout:** Vertical scroll, `space-y-2` between cards.
- **Pinning:** Selected card always rendered first, regardless of sort order.
- **Separator:** When pinned exists AND `rest.length > 0`, a `— others —` divider row is inserted.

### Zone 3: Session card
- **Padding:** `px-3 py-2`
- **Border:** Conditional emerald-700 (selected) or zinc-800 (default).
- **Hover icons:** `top-2 right-2`, hidden by default, `group-hover:flex`.
- **Status row:** Single horizontal flex; auto-pill pushed to right via `ml-auto`.

---

## Flow Diagram

```
sessions[] (push from main)
  |
  +-- Drawer applies status-filter chips + search
  |
  +-- visible[] = sessions filtered
        |
        +-- pinned = visible.find(id === selectedId)
        +-- rest   = visible.filter(id !== selectedId)
              |
              +-- render pinned (if any) + separator + rest

User clicks card
  |
  +-- App.onSelect(id)
        |
        +-- s.setSelectedSessionId(id)
        +-- s.setSelectedFile(null)
        +-- (drawer re-renders with new pinned)

User clicks ⤴
  |
  +-- App.onFocusTerminal(id)
        |
        +-- look up session.cwd
        +-- call window.rcc.focusTerminal({cwd, sessionId, terminalApp, skipPermissions})
              |
              +-- (see contract §34-39 for terminalApp branching)

User flips AutoContinuePill
  |
  +-- SessionCard.flipAutoContinue
        |
        +-- toggle local state, persist localStorage
        +-- call onSetAutoContinue(newState) -> App.tsx handler
              |
              +-- find PTY by sessionId + alive
              +-- if found: write directive + \r
              +-- if not: emit toast
```

---

## Cross-References

| Concept | Spec |
|---|---|
| Auto-continue overall mechanic (launch + mid-run) | [`../launch-modal/spec.md`](../launch-modal/spec.md) |
| Workflows tab | [`../workflows-panel/spec.md`](../workflows-panel/spec.md) |
| What "Focus terminal" does in each terminalApp mode | [`../terminal/spec.md`](../terminal/spec.md) |
| Status state machine across hooks | [`../hooks/spec.md`](../hooks/spec.md) |
| Bottom panel that reacts to selection | [`../progress-panel/spec.md`](../progress-panel/spec.md) |

## Source Files

| File | Path |
|---|---|
| Drawer (tabs, filters, search, pinning) | `src/renderer/src/components/Drawer.tsx` |
| SessionCard (per-session render) | `src/renderer/src/components/SessionCard.tsx` |
| Sessions service (composition + status) | `src/main/services/sessions.ts` |
| Predecessor/successor linking | `src/main/services/sessions.ts:221` (propagateContinuations) |
| Session type | `src/shared/types.ts:40-63` |
| Context-window model lookup | `src/main/services/models.ts` |
| Auto-continue mid-run handler | `src/renderer/src/App.tsx` (`onSetAutoContinue` callback) |
