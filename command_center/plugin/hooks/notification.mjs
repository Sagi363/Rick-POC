#!/usr/bin/env node
import { readJsonStdin, readTracking, writeTracking, withFileLock } from './lib.mjs'

const ev = await readJsonStdin()
const sessionId = ev.session_id
if (!sessionId) process.exit(0)

await withFileLock(sessionId, async () => {
  const { frontmatter, sections } = await readTracking(sessionId)
  frontmatter.status = 'waiting'
  await writeTracking(sessionId, { frontmatter, sections })
})
