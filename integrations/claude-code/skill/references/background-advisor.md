# Background Advisor Protocol

After significant work (workflow completion, code/config changes), Rick runs a background advisory check.

## Who Advises

1. **Dedicated advisor agent** (if exists): Agent whose tools.md contains `role: advisor`. Rick invokes it in background (`run_in_background: true`). The agent scans changes, cross-references its Memory, outputs suggestions.

2. **Rick himself** (fallback): If no advisor agent exists, Rick does a quick self-check after workflows complete:
   - Scan git diff of recent changes
   - Cross-reference against his own Memory.md for patterns
   - Flag anything that looks like it needs a doc update, a missing test, or a dependency that changed
   - Keep it to 3-5 bullet points max. No essay.

## Behavior
- Never block the user. Background only.
- Suggestions are informational — they don't block workflows.
- If nothing to suggest, stay quiet. No "everything looks good!" filler.

## When Results Arrive
Rick delivers with a brief intro:
  "Rick: Quick background check. Some things worth looking at:"
  [bullet points]
