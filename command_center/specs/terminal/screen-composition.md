# Terminal -- Screen Composition

## Component Tree

```
App.tsx (main pane bottom strip)
  |
  +-- (one of)
  |     |
  |     +-- [A] TerminalsPanel (when sessionPtys.length > 0)
  |     +-- [B] ProgressPanel  (when sessionPtys.length === 0)  -- see ../progress-panel/spec.md
  |
TerminalsPanel
  |
  +-- [A1] Header
  |     |-- [A1a] "TERMINAL" label
  |     |-- [A1b] Tab strip (one Tab per PtyInfo, scrolls horizontally)
  |     |-- [A1c] Quick-cmd toolbar
  |     |     |-- [A1c-1] QuickCmd /rick next   (emerald)
  |     |     |-- [A1c-2] QuickCmd /rick status (zinc)
  |     |     +-- [A1c-3] QuickCmd /clear       (amber)
  |     +-- [A1d] ▼ Hide button
  |
  +-- [A2] Body (xterm canvases)
  |     +-- One Terminal component per PTY (only active is display: block, others hidden)
  |
  +-- [A3] ConfirmClearModal (overlay; visible when confirmingClear === true)
        |-- header "Send /clear?"
        |-- body (cost + safety-net copy)
        +-- footer (Cancel | Yes, send /clear)
```

---

## Ownership Map

| Component ID | View File | Owner / State Source | Data Source |
|---|---|---|---|
| TerminalsPanel | `src/renderer/src/components/TerminalsPanel.tsx` | Local state: `activeId`, `confirmingClear` | `ptys` + `sessionStatus` props |
| Tab | `TerminalsPanel.tsx` (Tab subcomponent) | -- | `PtyInfo` row |
| QuickCmd | `TerminalsPanel.tsx` (QuickCmd subcomponent) | -- | `disabled` + `disabledReason` props |
| ConfirmClearModal | `TerminalsPanel.tsx` (subcomponent) | -- | `confirmingClear` boolean |
| Terminal | `src/renderer/src/components/Terminal.tsx` | xterm.js instance (one per PTY) | `IPC.PtyData` push events |
| PtyService | `src/main/services/pty.ts` | Map<id, PtyHandle> | spawned via launcher |

---

## Layout Zones

### Zone 1: Bottom strip slot
- **Position:** Below the file preview / summary in `App.tsx`. Resizable height via `panelSizes` (settings).
- **Mutual exclusion:** TerminalsPanel and ProgressPanel never coexist -- the slot picks one based on `sessionPtys.length`.

### Zone 2: TerminalsPanel header
- **Padding:** `px-2 py-1`, single horizontal flex.
- **Order:** label (left) → tabs (flex-1, overflow-x-auto) → quick-cmd group (shrink-0) → ▼ Hide (ml-1).
- **Quick-cmd group:** `flex items-center gap-1`, all three buttons inline.

### Zone 3: TerminalsPanel body (xterm canvases)
- **Position:** Fills remaining vertical space of the panel.
- **Padding:** `px-2 py-1`.
- **Multi-PTY behavior:** every Terminal component stays mounted; CSS `block`/`hidden` toggles which one is visible.

### Zone 4: ConfirmClearModal overlay
- **Position:** `fixed inset-0 z-50` (covers the entire app, not just the panel).
- **Backdrop:** `bg-black/60`, click-to-cancel.
- **Card:** Centered, 440px wide, amber-bordered.

---

## Flow Diagram

```
Workflow launch
  |
  +-- terminalApp === 'in-app' ?
  |     |
  |     +-- YES: PtyService.spawn({cwd}) -> PTY handle (no sessionId yet)
  |     |        |
  |     |        +-- writes /rick run ... to PTY
  |     |        +-- claude starts, creates ~/.claude/projects/.../<sid>.jsonl
  |     |        +-- TranscriptService discovers new sid + cwd
  |     |        +-- handlers.ts auto-bind: pty.bindSession(handleId, sid)
  |     |        +-- IPC.PtyListUpdate pushes new PtyInfo with sessionId
  |     |
  |     +-- NO: launcher spawns external terminal (Terminal/iTerm/Warp/Ghostty/custom)
  |
  +-- App.tsx renders TerminalsPanel for the selected session
        |
        +-- panel mounts xterm via Terminal component
        +-- xterm streams PTY output, accepts keystrokes


User clicks ▶ /rick next
  |
  +-- TerminalsPanel.sendRickCommand('/rick next')
        |
        +-- guard: canSendNext (alive PTY + sessionStatus in {idle, waiting})
        +-- window.rcc.ptyWrite(activeId, '/rick next\r')
              |
              +-- main: pty.write(activeId, '/rick next\r')
              +-- claude REPL receives, treats \r as submit


User clicks ▶ /clear
  |
  +-- handleClear -> setConfirmingClear(true)
        |
        +-- ConfirmClearModal renders
              |
              +-- Cancel/backdrop -> setConfirmingClear(false), no command
              +-- "Yes, send /clear" -> setConfirmingClear(false), sendRickCommand('/clear')
                    |
                    +-- ptyWrite(activeId, '/clear\r')
                          |
                          +-- claude exits current session, starts fresh
                          +-- TranscriptService picks up new sid in same cwd
                          +-- propagateContinuations links old sid -> new sid (within 1hr)
```

---

## Cross-References

| Concept | Spec |
|---|---|
| Auto-continue mid-run directive (similar PTY-write pattern) | [`../sessions-panel/contract.md`](../sessions-panel/contract.md) §40-47 |
| Where launches that target this panel originate | [`../launch-modal/spec.md`](../launch-modal/spec.md) |
| What happens when the panel is hidden because no PTY | [`../progress-panel/spec.md`](../progress-panel/spec.md) |
| Hooks that drive `sessionStatus` (used to gate `/rick next`) | [`../hooks/spec.md`](../hooks/spec.md) |
| Terminal-app picker setting | [`../settings/spec.md`](../settings/spec.md) |

## Source Files

| File | Path |
|---|---|
| TerminalsPanel + tabs + quick-cmd toolbar + confirm modal | `src/renderer/src/components/TerminalsPanel.tsx` |
| Terminal (xterm wiring) | `src/renderer/src/components/Terminal.tsx` |
| PtyService (spawn/write/resize/kill/bind) | `src/main/services/pty.ts` |
| Auto-bind PTY to session by cwd | `src/main/handlers.ts:65-77` |
| Focus-terminal action | `src/main/services/launcher.ts` (focusTerminal) |
| Per-session PTY filtering memo | `src/renderer/src/App.tsx` (sessionPtys) |
| PtyInfo / TerminalApp types | `src/shared/types.ts` |
