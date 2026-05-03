#!/usr/bin/env node
import { readJsonStdin, readTracking, writeTracking, withFileLock } from './lib.mjs'

const ev = await readJsonStdin()
const sessionId = ev.session_id
if (!sessionId) process.exit(0)

const prompt = (ev.prompt ?? '').toString()
const m = prompt.match(/^\/rick\s+run\s+([^\s]+)/m)

await withFileLock(sessionId, async () => {
  const { frontmatter, sections } = await readTracking(sessionId)
  if (m && !frontmatter.workflow) frontmatter.workflow = m[1]
  if (!frontmatter.status) frontmatter.status = 'running'
  await writeTracking(sessionId, { frontmatter, sections })
})
