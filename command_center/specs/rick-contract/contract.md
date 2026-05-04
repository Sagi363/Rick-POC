# Rick ↔ Command Center Protocol -- Contract

The exact regex contracts, parser pass rules, and signal grammar. Both sides (Rick's persona and the correlator) must implement these verbatim.

---

## Signal Grammar (Rick MUST emit)

1. Phase markers SHALL appear on their own line, NOT wrapped in prose: `[rick:phase <step-id> <starting|complete>]`.
2. Phase marker `<step-id>` SHALL match `[a-z][\w-]*` and SHALL equal the literal `id:` of an outer step in the workflow YAML.
3. Phase marker keyword `rick:phase` SHALL be lowercase. Brackets are literal.
4. Phase marker status SHALL be `starting` or `complete` (lowercase preferred; case-insensitive on parse side).
5. The workflow-completion banner SHALL be of form `Rick: All <N> steps complete` (case-insensitive, leading `**Rick:**` allowed).
6. Handoff lines SHALL include the subagent name in `**bold**`. The `(Role)` parenthetical is OPTIONAL.
7. Handoff lines SHALL separate the subagent from the activity with an em-dash `—`, en-dash `–`, or hyphen `-`.
8. Handoff lines MAY include a trailing `[claude:<model>]` annotation; it SHALL NOT appear inside the activity description.
9. Phase prose lines SHALL match `\bPhase\s+<N>\s+<verb>\b` where `<verb>` ∈ {`starting`, `started`, `begins`, `begin`, `done`, `complete`, `finished`}.
10. Future-tense / descriptive forms (`will`, `=`, `Waiting on`, `What Phase X will do`) SHALL NOT be parsed as transitions.

## Regex Contracts (correlator MUST implement)

11. `HANDOFF_RE = /Rick:\s*\*{0,2}\s*Handing\s+to/gi` -- counts handoffs (Pass 3 fallback).
12. `DONE_RE = /Rick:\s*\*{0,2}\s*[A-Za-z][\w-]*\s+is\s+done/gi` -- counts agent-done announcements (Pass 3 fallback).
13. `COMPLETE_RE = /Rick:\s*\*{0,2}\s*All\s+\d+\s+steps?\s+complete/i` -- workflow-completion banner. **Mirrored in `plugin/hooks/stop.mjs`**.
14. `PHASE_RE = /\[rick:phase\s+([a-z][\w-]*)\s+(starting|complete)\]/gi` -- bracket markers (Pass 2).
15. `PHASE_PROSE_RE = /\bPhase\s+(\d+)\s+(starting|started|begins?|done|complete|finished)\b/gi` -- prose fallback (Pass 2.5).
16. `HANDOFF_DETAIL_RE = /Handing\s+to\s+\*+([^*\n]+?)\*+(?:\s*\(([^)\n]+?)\))?\s*[—–-]+\s*([^\n[]+?)(?:\s*\[|$)/i` -- handoff details for live activity.

## Parser Pass Order

17. Pass 1 SHALL run unconditionally and use direct subagent-name matching against `step.agent` / `step.collaborators`.
18. Pass 2 SHALL fire when ≥ 1 phase marker is present.
19. Pass 2 SHALL be applied AFTER Pass 1 -- markers override Pass 1 status for the same step (markers are explicit; subagent matching is heuristic).
20. Pass 2.5 SHALL fire ONLY when Pass 2 produced no signal (`markersApplied === false`).
21. Pass 2.5 SHALL be applied AFTER Pass 1.
22. Pass 3 SHALL fire ONLY when NONE of Pass 1, 2, 2.5 produced any signal.
23. After all passes, the correlator SHALL scan for the latest handoff-detail line and attach `currentSubagent` / `currentActivity` to the highest-indexed `running` step.

## Pass 1 -- Subagent Name Matching

24. The correlator SHALL iterate every main-thread `Task` tool_use in the JSONL.
25. The `subagent_type` SHALL be normalized: when prefixed `rick-`, take the substring after the LAST `-` (e.g. `rick-Issues-Team-sherlock` → `sherlock`).
26. The normalized agent SHALL be matched against `step.agent` (exact) OR `step.collaborators.includes(norm)` OR `norm.endsWith(step.agent)`.

## Pass 2 -- Phase Markers

27. Phase marker step-id SHALL be lowercased before matching against the YAML id.
28. Markers for unknown step ids SHALL be silently skipped.
29. A `complete` marker SHALL set status to `done` regardless of prior status.
30. A `starting` marker SHALL set status to `running` ONLY if the step is not already `done`.

## Pass 2.5 -- Phase Prose

31. Phase number SHALL map to step index `phaseNum - 1`.
32. Out-of-range phase numbers SHALL be silently skipped.
33. After applying all prose markers, the correlator SHALL identify the highest-numbered `complete` and -- when its `phaseNum < steps.length` -- SHALL flip step `phaseNum` to `running` ONLY if currently `pending`. (Rick often skips "Phase N+1 starting".)

## Pass 3 -- Announcement Counting

34. `completed = min(announcements.completions, steps.length)`.
35. Steps `[0..completed)` SHALL be marked `done`.
36. When `inFlight === true && completed < steps.length`, step `completed` SHALL be marked `running`.
37. `inFlight = !allComplete && handoffs > completions`.
38. `allComplete` is true when COMPLETE_RE matched anywhere in the transcript.

## Live Activity Attribution

39. The correlator SHALL iterate `steps[]` from highest index to lowest after all passes.
40. The first step found with `status === 'running'` SHALL receive `currentSubagent` and `currentActivity`.
41. `currentSubagent` SHALL be `<name>` when no role captured, or `<name> (<role>)` when role captured.
42. The handoff line LAST occurring in the transcript SHALL be the source -- not the first.

## Stop Hook Mirror

43. `plugin/hooks/stop.mjs` SHALL implement `COMPLETE_RE` IDENTICALLY to `correlation.ts`.
44. When `COMPLETE_RE` is changed in correlation.ts, the same change SHALL be propagated to stop.mjs in the same commit.

## Persona Mirror

45. `~/.rick/persona/rules.md` SHALL document the phase-marker contract under a "Phase Markers" section.
46. The persona description SHALL match the regexes -- e.g. saying "lowercase" matches `[a-z][\w-]*`, saying "either em-dash or hyphen" matches `[—–-]+`.
47. When new signals are added, `rules.md` SHALL be updated BEFORE Rick's persona is reloaded.

---

## Open Questions

- OQ1: Should `[rick:phase <id> blocked]` be a third valid state for `track blocked`?
- OQ2: Should the correlator log which pass fired (per step) for diagnostic purposes?
- OQ3: Should we add a structured `[rick:auto on|off]` directive form so auto-continue stops relying on natural-language Rick instructions?
- OQ4: Should phase-marker step-id matching tolerate snake_case vs kebab-case variants automatically?
