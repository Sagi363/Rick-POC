# Progress / Activity Panel -- Presentation Spec

## 1. Overview

The bottom strip of the main pane. Shows EITHER the rendered tracking-file body (Mode A) OR a flattened activity feed of the recent transcript (Mode B), depending on whether tracking has structured progress data.

The panel hard-hides when an in-app PTY is alive for the selected session -- the [Terminal panel](../terminal/spec.md) takes the slot. They share the same vertical real estate and never coexist.

**Source files:**
- `src/renderer/src/components/ProgressPanel.tsx` (full file) -- Component
- `src/main/services/tracking.ts` -- readTracking via IPC
- `src/main/services/summary.ts` -- summarizeTranscript via IPC
- `src/renderer/src/App.tsx` (the `hasPtys` branch around the bottom strip)

---

## 2. Presentation Models

### 2.1 ProgressPanelState

Local renderer state. Conforms to: Equatable.

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `body` | String | No | `"## Todos\n- [x] …"` | dynamic -- raw markdown body of tracking.md, frontmatter stripped |
| `summary` | SessionSummary | Yes | (see 2.2) | dynamic -- only fetched in Mode B |
| `loadingSummary` | Bool | No | `false` | true during summary fetch |

### 2.2 SessionSummary (subset relevant here)

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `recent` | List of RecentMessage | No | (see 2.3) | Up to 12 most-recent main-thread events |

### 2.3 RecentMessage (Mode B feed item)

| Field | Type | Optional | Example | Notes |
|---|---|---|---|---|
| `kind` | "user" \| "reply" \| "tool" \| "agent" | No | `"reply"` | Source category (see 3.2) |
| `text` | String | No | `"Phase 2 starting…"` | Single-line, max 240 chars |
| `timestamp` | Number (ms) | No | `1746284123000` | From the JSONL line's timestamp |

### 2.4 hasTrackedProgress (boolean derivation)

| Source | Result |
|---|---|
| `session.total != null` OR `session.phase != null` OR `session.current != null` | `true` -- Mode A |
| All three null | `false` -- Mode B |

---

## 3. Visual States

### 3.1 Mode A: Tracked progress

| Region | Content |
|---|---|
| Header | "Progress" label + optional `phase: <name>` chip + ▼ Hide |
| Header progress bar | When `total > 0`: emerald fill bar with `<completed>/<total>` and percent labels |
| Header current line | When `current` set: `→ <current todo text>` |
| Body | Markdown render of tracking body (`## Todos`, `## Phase log`, `## Artifacts`) |

### 3.2 Mode B: Activity feed

Up to 12 most-recent events, each as one row: `<HH:MM:SS> [KIND] <text>`.

| Kind | Source | Tint |
|---|---|---|
| `user` | type=user message text | blue (`bg-blue-900/60 text-blue-200`) |
| `reply` | type=assistant text content | emerald |
| `tool` | type=assistant tool_use (name != Task) | amber |
| `agent` | type=assistant tool_use (name == Task) | purple |

### 3.3 Empty / loading states

| Condition | Content |
|---|---|
| No session selected | "Pick a session." |
| Mode A but body is empty | "Tracking exists but body is empty." |
| Mode B with no events | "No activity yet." |
| Loading | "Reading transcript…" |

### 3.4 Collapsed strip

When collapsed, the panel becomes a one-line strip: `Activity hidden` + `▲ Show`.

---

## 4. Interactions

| Target | Gesture | Result |
|---|---|---|
| ▼ Hide | Click | Collapses panel to strip |
| ▲ Show | Click | Re-expands |

---

## 5. Auto-Hide

The panel hard-hides whenever `sessionPtys.length > 0` for the selected session. The TerminalsPanel takes the slot. When PTY count drops to zero, the panel re-appears at the previous collapsed state.

The Activity feed used to also live inside the Summary panel; that was removed (PROGRESS.md notes — ~M2 wrap-up) because it duplicated the bottom panel's content.

---

## 6. Data Fetch Flow

```
session changes OR session.lastActivity changes OR collapsed flips false
  |
  +-- readTracking(session.id) -> setBody (markdown body, frontmatter stripped)
  |
  +-- (if !hasTrackedProgress)
        |
        +-- getSessionSummary(session.id) -> setSummary
              |
              +-- summarizeTranscript: iterate JSONL, last 12 main-thread events
```

`loadingSummary` is true while the summary fetch is in flight. Mode A's body never shows a "loading" state -- it's a single fast read of a small markdown file.

---

## Open Questions

- OQ1: Should Mode B show MORE than 12 events with a scroll, instead of hard-cap at 12?
- OQ2: When tracking has progress AND there's also recent transcript activity, should the panel show a "tabs" of Mode A and Mode B?
- OQ3: Should the tracking body re-render via a markdown library that supports task-list checkboxes interactively (so the user can check items)?
