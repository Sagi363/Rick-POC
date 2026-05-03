# Rick ↔ Command Center Protocol -- Spec

## 1. Overview

The protocol between Rick (the orchestrator persona at `~/.rick/persona/`) and the correlator inside the Command Center (`src/main/services/correlation.ts`).

Rick produces structured signals in main-thread assistant text. The Command Center reads those signals out of `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl` and turns them into the workflow-progress UI.

This is a **two-sided contract** — both ends must stay in sync. Rick's persona file (`~/.rick/persona/rules.md`) declares what to emit. The correlator's regexes parse it. If you change one without the other, the UI silently degrades.

**Source files:**
- `src/main/services/correlation.ts` (full file) -- The parser
- `~/.rick/persona/rules.md` (Phase Markers section) -- The persona contract Rick reads at session start
- `src/shared/types.ts` -- WorkflowRunView, WorkflowStepView (output shape)

---

## 2. The Four Signals

Rick emits four kinds of structured signals into main-thread assistant text. The correlator picks them up.

### 2.1 Phase markers (REQUIRED for multi-step / `uses:` composed workflows)

Bracket markers Rick emits on its own line at every outer-step boundary:

```
[rick:phase <step-id> starting]
... step runs ...
[rick:phase <step-id> complete]
```

`<step-id>` matches the `id:` of the outer step in the workflow YAML. The correlator's `PHASE_RE` is the load-bearing regex.

**Why mandatory for composed workflows:** in a `uses:`-chained workflow, the outer step's `agent` is a sub-workflow name (e.g. `ticket-research`) and never appears as a `subagent_type`. Pass-1 (subagent matching) cannot help. Inner `Handing to …` lines fire many times per outer step, so counting them mis-attributes progress (Pass-3 fallback). Bracket markers are the only deterministic per-outer-step signal.

### 2.2 Workflow-completion banner (REQUIRED at end-of-workflow)

```
Rick: All <N> steps complete
```

The Stop hook reads the latest assistant message and tests it against `COMPLETE_RE`. **Only a match flips tracking `status: done`.** Without the banner, Stop is a no-op.

### 2.3 Live activity -- handoff details (REQUIRED for visibility into running phase)

Each subagent handoff line:

```
**Rick:** Handing to **<Subagent>** (<Role>) — <activity> [claude:<model>]
```

Components:
- Bold around `Rick:` is optional; the literal `Rick:` is required.
- Subagent name MUST be wrapped in `**bold**`.
- `(<Role>)` is optional.
- An em-dash, en-dash, or hyphen separates the agent from the activity description.
- Trailing `[claude:<model>]` is optional and stripped from the parsed activity.

The correlator extracts the **most recent** handoff line and attaches `currentSubagent` (with role appended in parens) and `currentActivity` to whichever outer step is currently running.

### 2.4 Phase prose (FALLBACK -- accepted but not preferred)

For backward compatibility with Rick personas that pre-date the bracket-marker contract:

```
Phase <N> <verb>
```

Where `<verb>` is one of: `starting`, `started`, `begins`, `begin`, `done`, `complete`, `finished`.

Rejected (intentional -- these are descriptive, not transitional):
- `Phase 2 will`
- `Phase 2 = reproduce`
- `Waiting on Phase 2`
- `What Phase 2 will do`
- `Phase 1 has one more parallel agent`

This fallback exists because re-prompting an in-flight session to pick up persona changes is fragile. New work SHOULD emit bracket markers.

---

## 3. The Correlator -- Four Passes

The correlator runs four ordered passes per `correlateWorkflow` call. Later passes override earlier ones for the same step.

| Pass | Source | Status output | When it fires |
|---|---|---|---|
| 1 | Main-thread `Task.subagent_type` matched against `step.agent` / `step.collaborators` | `running` / `done` per matched step | Always; no-op for `uses:` composed workflows |
| 2 | `[rick:phase …]` markers (`PHASE_RE`) | `running` (on `starting`) / `done` (on `complete`) | When ≥ 1 marker is in the transcript |
| 2.5 | Phase prose (`PHASE_PROSE_RE`) | Same as Pass 2; phase number → step index | Only when Pass 2 produced no signal |
| 3 | `Handing to …` / `… is done` count totals | `done` for first N steps; running for N+1 | Only when none of 1, 2, 2.5 produced any signal |

After all passes, the correlator scans for the latest handoff-detail line and attaches `currentSubagent` / `currentActivity` to the highest-indexed running step.

---

## 4. Command Center → Rick directives

The Command Center sends these via the in-app PTY (xterm.js + node-pty). Each is appended with `\r` (carriage return) to submit.

### 4.1 Quick commands (terminal toolbar)

| Button label | What's actually sent | Why |
|---|---|---|
| `▶ /rick next` | `/rick next\r` | Resume after `auto_continue: false` pause |
| `▶ /rick status` | `/btw rick status\r` | Out-of-band status query — `/btw` prevents interrupting Rick's current turn |
| `▶ /clear` | `/clear\r` (after confirmation modal) | End current Claude session; successor-detection in the app handles carryover |

### 4.2 Auto-continue overrides

**At launch** (composed into the `extraPrompt` of the initial `/rick run …` message):

> `Override: run all phases with auto_continue: true — do not pause between phases or wait for me to say next. Drive the workflow end-to-end.`

**Mid-run** (sent as a `/btw …` directive when the session-card pill is flipped):

| Pill state | Directive sent |
|---|---|
| ON | `/btw From now on, run remaining phases with auto_continue: true — do not pause between phases or wait for me to say next. Drive the workflow end-to-end.` |
| OFF | `/btw From now on, run remaining phases with auto_continue: false — pause after each phase and wait for my next before continuing.` |

Rick is expected to honor these as overrides to per-step `auto_continue:` flags in the workflow YAML for the **remaining** phases.

---

## 5. Versioning

Today's contract is implicit-v1. If new signals are added (e.g. `[rick:phase <id> blocked]`, or `[rick:agent <name> starting/done]` for finer-grained sub-step tracking), bump this doc with a v2 section and keep v1 regexes as fallbacks for older Rick personas in flight.

---

## 6. Cross-References

| Concern | Spec |
|---|---|
| Detailed regex contracts (verbatim) | [`contract.md`](contract.md) |
| Where the Stop hook uses COMPLETE_RE | [`../hooks/spec.md`](../hooks/spec.md) §3.4 |
| Where currentSubagent / currentActivity render | [`../workflow-status/spec.md`](../workflow-status/spec.md) §3.4 |
| Auto-continue source UI | [`../launch-modal/spec.md`](../launch-modal/spec.md) §7 + [`../sessions-panel/spec.md`](../sessions-panel/spec.md) §2.6 |

## Source Files

| File | Path |
|---|---|
| Correlator passes + regexes | `src/main/services/correlation.ts` |
| WorkflowStepView shape | `src/shared/types.ts` |
| Stop hook (workflow-aware completion) | `plugin/hooks/stop.mjs` |
| Quick-command buttons + confirm modal | `src/renderer/src/components/TerminalsPanel.tsx` |
| Auto-continue launch directive | `src/renderer/src/components/LaunchModal.tsx` (`buildAutoContinueDirective`) |
| Auto-continue mid-run directive | `src/renderer/src/App.tsx` (`onSetAutoContinue` callback wiring) |
| Rick's side of the contract | `~/.rick/persona/rules.md` (Phase Markers section) |

---

## Open Questions

- OQ1: Should we add `[rick:phase <id> blocked]` as a third state for `track blocked`?
- OQ2: Should auto-continue directives have a structured `[rick:auto on|off]` form so we don't rely on natural-language parsing on Rick's side?
- OQ3: Should the correlator emit metrics (pass that fired, regex match counts) for debugging?
