export type SessionStatus = 'running' | 'waiting' | 'blocked' | 'done' | 'idle'

export interface Universe {
  name: string
  path: string
}

export interface WorkflowParam {
  name: string
  type: 'string' | 'int' | 'bool' | 'enum' | 'unknown'
  default?: unknown
  description?: string
  required?: boolean
  enumValues?: string[]
}

export interface WorkflowStep {
  id: string
  agent: string
  collaborators?: string[]
  description?: string
  dependsOn?: string[]
  uses?: string
}

export interface Workflow {
  name: string
  universe: string
  filePath: string
  description?: string
  agents: string[]
  dependsOn: string[]
  params: WorkflowParam[]
  steps: WorkflowStep[]
}

export interface SessionContext {
  used: number
  limit: number
  model: string
  modelKnown: boolean
}

export interface Session {
  id: string
  title?: string
  customTitle?: string
  workflow?: string
  universe?: string
  cwd: string
  transcriptPath: string
  trackingPath?: string
  status: SessionStatus
  phase?: string
  total?: number
  completed?: number
  current?: string
  context?: SessionContext
  lastActivity: number
  startedAt?: number
  successorId?: string
  predecessorId?: string
}

export interface AppSettings {
  warnThreshold: number
  criticalThreshold: number
  recentSessionDays: number
  lastUniverse?: string
  panelSizes?: Record<string, number>
  pluginInstalled?: boolean
  archivedSessionIds?: string[]
  branchPrefix: string
  defaultBranchOff: string
  terminalApp: TerminalApp
  customTerminalCommand?: string
  skipPermissions: boolean
  customTitles?: Record<string, string>
}

export type TerminalApp = 'in-app' | 'Terminal' | 'iTerm' | 'Warp' | 'Ghostty' | 'custom'

export const DEFAULT_SETTINGS: AppSettings = {
  warnThreshold: 0.70,
  criticalThreshold: 0.90,
  recentSessionDays: 7,
  archivedSessionIds: [],
  branchPrefix: 'feature/',
  defaultBranchOff: 'dev',
  terminalApp: 'Terminal',
  skipPermissions: false
}

export interface WorkflowStepView {
  id: string
  agent: string
  collaborators: string[]
  description?: string
  status: 'pending' | 'running' | 'done'
  files: string[]
  startedAt?: number
  /** Live sub-activity attributed to the currently-running step (latest Rick
   *  "Handing to <Subagent> — <activity>" line). Replaces the generic
   *  agent/sub-workflow name in the UI while the step is running. */
  currentSubagent?: string
  currentActivity?: string
}

export interface WorkflowRunView {
  workflow: string
  label: string
  feature?: string
  steps: WorkflowStepView[]
}

export interface WorktreeRequest {
  base: string
  name: string
  branch: string
  fromBranch?: string
}

export interface LaunchRequest {
  workflow: string
  universe: string
  cwd: string
  params: Record<string, unknown>
  extraPrompt?: string
  worktree?: WorktreeRequest
  terminalApp?: TerminalApp
  customTerminalCommand?: string
  skipPermissions?: boolean
}

export interface FocusTerminalRequest {
  cwd: string
  sessionId: string
  terminalApp: TerminalApp
  customTerminalCommand?: string
  skipPermissions?: boolean
}

export interface FocusTerminalResult {
  ok: boolean
  error?: string
  resumed?: boolean
}

export interface PtyInfo {
  id: string
  cwd: string
  label: string
  pid: number
  startedAt: number
  alive: boolean
  exitCode?: number
  sessionId?: string
}

export interface SpawnInAppRequest {
  cwd: string
  sessionId?: string
  resume?: boolean
  initialPrompt?: string
  label?: string
  skipPermissions?: boolean
}

export interface LaunchResult {
  ok: boolean
  error?: string
  command?: string
  existingWorktreePath?: string
}

export interface SessionFilesDTO {
  tracking?: string
  specs: string[]
  touched: string[]
  transcript?: string
  workflowRun?: WorkflowRunView
}

export interface SessionSummaryEntry {
  type: 'user' | 'assistant'
  timestamp: number
  text?: string
  toolUses: { name: string; brief: string }[]
}

export interface SessionSummary {
  id: string
  workflow?: string
  cwd: string
  model?: string
  startedAt?: number
  lastActivity: number
  totalMessages: number
  totalUserMessages: number
  totalAssistantMessages: number
  toolCounts: Record<string, number>
  subagentSpawns: { name: string; description: string; timestamp: number }[]
  recent: SessionSummaryEntry[]
  contextHistory: { timestamp: number; used: number; limit: number }[]
  touchedFiles: string[]
}
