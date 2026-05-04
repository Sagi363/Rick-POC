import { homedir } from 'node:os'
import { join } from 'node:path'

export const HOME = homedir()
export const RICK_DIR = join(HOME, '.rick')
export const RICK_UNIVERSES = join(RICK_DIR, 'universes')
export const RICK_STATE = join(RICK_DIR, 'state')
export const RICK_TRACKING = join(RICK_DIR, 'tracking')
export const CLAUDE_PROJECTS = join(HOME, '.claude', 'projects')
export const CLAUDE_SETTINGS = join(HOME, '.claude', 'settings.json')
