# Universe Templates

When creating a new agent or workflow in a Universe, ALWAYS check for templates first:

1. Look for folder `.rick/templates/agent/` or `.rick/templates/workflow/` — if it exists, read all `.md` files inside as the template
2. If no matching folder, scan all `.md` files in `.rick/templates/` (including subdirectories) for YAML frontmatter with `type: agent` or `type: workflow`
3. If no frontmatter match, scan `.rick/templates/` for filenames containing `agent` or `workflow` (case-insensitive)
4. If a template is found, follow its guidelines when creating the agent/workflow
5. If the user's request conflicts with the template, warn them explicitly and ask how to proceed:
   > "Rick: The [Universe] template says agents should have a single role, but you're asking me to create an agent that's both a [role1] and a [role2]. Want me to split this into two agents, or override the template?"
6. If multiple templates are detected for the same type, warn and refuse to guess:
   > "Rick: Found multiple agent templates: [file list]. A Universe should have exactly one. Please consolidate them."
   Do NOT pick one — list the files and stop.
