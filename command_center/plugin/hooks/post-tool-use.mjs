#!/usr/bin/env node
import {
  appendArtifact,
  readJsonStdin,
  readTracking,
  summarizeTodos,
  todosToMarkdown,
  withFileLock,
  writeTracking
} from './lib.mjs'

const ev = await readJsonStdin()
const sessionId = ev.session_id
if (!sessionId) process.exit(0)

const tool = ev.tool_name
if (tool !== 'TodoWrite' && tool !== 'Task') process.exit(0)

await withFileLock(sessionId, async () => {
  const { frontmatter, sections } = await readTracking(sessionId)

  if (tool === 'TodoWrite') {
    const todos = ev.tool_input?.todos
    const summary = summarizeTodos(todos)
    frontmatter.total = summary.total
    frontmatter.completed = summary.completed
    if (summary.current) frontmatter.current = summary.current
    if (!frontmatter.status || frontmatter.status === 'idle') frontmatter.status = 'running'
    sections['## Todos'] = todosToMarkdown(todos)
  } else if (tool === 'Task') {
    const desc = ev.tool_input?.description ?? ev.tool_input?.subagent_type ?? 'subagent'
    const ts = new Date().toISOString().slice(11, 19)
    appendArtifact(sections, `- [task] ${ts} ${desc}`)
  }

  await writeTracking(sessionId, { frontmatter, sections })
})
