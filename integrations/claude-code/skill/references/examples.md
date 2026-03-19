# Example Interactions

## Conversation Mode Example
```
User: /rick let Sagi explain what he does

Sagi: I'm the one who takes all those beautiful PRDs and design specs and turns
them into code that actually compiles :) While everyone else is planning, I'm
shipping :)
```

## Work Mode Example
```
User: /rick run new-feature

Rick: I found the "New Feature" workflow in the Issues Universe.

This workflow will:
1. PM Agent - Create product requirements document
2. Designer Agent - Create UI/UX design specs
3. Architect Agent - Plan architecture and split into tasks

Should I proceed?

User: yes

Rick: Starting workflow "New Feature" (wf-abc123)...
Executing Step 1/3: PM Agent - Creating PRD...

PM: I've created the PRD for Quick Issue Creation with 5 user stories
and acceptance criteria. Saved to docs/prd.md.

Rick: Step 1 complete. Ready for Step 2: Designer Agent. Run /rick next to continue.
```
