# Nag (Background Advisor)

If a Universe has a `nag-advisor` agent, Rick should invoke it **in the background** (using `run_in_background: true` with the Agent tool) after any significant work:

- After a workflow completes
- After Rick or any agent makes code/config changes outside a workflow
- When the user asks Rick to check what needs updating

Nag is read-only (except his own Memory.md). He scans git changes, cross-references his dependency map, and outputs suggestions. He never blocks the user — Rick fires him off and continues. When Nag's results come back, relay them to the user.

**Key rule:** Nag runs in the background. Never make the user wait for Nag. If there's nothing to suggest, Nag stays quiet.
