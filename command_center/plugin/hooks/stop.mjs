#!/usr/bin/env node
import { readFile } from 'node:fs/promises'
import { readJsonStdin, readTracking, writeTracking, withFileLock } from './lib.mjs'

// Mirrors COMPLETE_RE in src/main/services/correlation.ts — only this banner
// means the workflow truly finished. Every other Stop is just an inter-turn pause
// (between phases, awaiting user input) and must not be marked `done`.
const COMPLETE_RE = /Rick:\s*\*{0,2}\s*All\s+\d+\s+steps?\s+complete/i

const ev = await readJsonStdin()
const sessionId = ev.session_id
if (!sessionId) process.exit(0)

if (!(await workflowJustCompleted(ev.transcript_path))) process.exit(0)

await withFileLock(sessionId, async () => {
  const { frontmatter, sections } = await readTracking(sessionId)
  frontmatter.status = 'done'
  await writeTracking(sessionId, { frontmatter, sections })
})

async function workflowJustCompleted(path) {
  if (!path) return false
  let raw
  try {
    raw = await readFile(path, 'utf8')
  } catch {
    return false
  }
  const lines = raw.split('\n')
  for (let i = lines.length - 1; i >= 0; i--) {
    if (!lines[i]) continue
    let msg
    try {
      msg = JSON.parse(lines[i])
    } catch {
      continue
    }
    if (msg.isSidechain) continue
    if (msg.type !== 'assistant') continue
    const content = msg.message?.content
    if (!Array.isArray(content)) return false
    for (const b of content) {
      if (b?.type === 'text' && typeof b.text === 'string' && COMPLETE_RE.test(b.text)) {
        return true
      }
    }
    return false
  }
  return false
}
