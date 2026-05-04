import type {
  AppSettings,
  FocusTerminalRequest,
  FocusTerminalResult,
  LaunchRequest,
  LaunchResult,
  PtyInfo,
  Session,
  SessionFilesDTO,
  SessionSummary,
  SpawnInAppRequest,
  Universe,
  Workflow
} from './types'

export interface InstallStatusDTO {
  installed: boolean
  pluginSourceDir: string
  pluginInstallDir: string
  hookCommands: Record<string, string>
  diff: { before: string; after: string }
}

export const IPC = {
  // Renderer → main (invoke)
  GetUniverses: 'rcc:universes:get',
  GetWorkflows: 'rcc:workflows:get',
  GetSessions: 'rcc:sessions:get',
  GetSettings: 'rcc:settings:get',
  SetSettings: 'rcc:settings:set',
  ReadFile: 'rcc:file:read',
  ListSessionFiles: 'rcc:session:files',
  ReadTracking: 'rcc:tracking:read',
  GetInstallStatus: 'rcc:install:status',
  RunInstall: 'rcc:install:run',
  RunUninstall: 'rcc:install:remove',
  DiscardSession: 'rcc:session:discard',
  UnarchiveSession: 'rcc:session:unarchive',
  GetSessionSummary: 'rcc:session:summary',
  PickDirectory: 'rcc:dialog:pick-dir',
  LaunchWorkflow: 'rcc:launch:workflow',
  FocusTerminal: 'rcc:terminal:focus',
  PtyList: 'rcc:pty:list',
  PtySpawn: 'rcc:pty:spawn',
  PtyWrite: 'rcc:pty:write',
  PtyResize: 'rcc:pty:resize',
  PtyKill: 'rcc:pty:kill',
  PtyBuffer: 'rcc:pty:buffer',
  PtyData: 'rcc:pty:data',
  PtyExit: 'rcc:pty:exit',
  PtyListUpdate: 'rcc:pty:list-update',

  // Main → renderer (events)
  UniversesUpdate: 'rcc:universes:update',
  WorkflowsUpdate: 'rcc:workflows:update',
  SessionsUpdate: 'rcc:sessions:update',
  FileChanged: 'rcc:file:changed'
} as const

export interface RccApi {
  getUniverses(): Promise<Universe[]>
  getWorkflows(universe?: string): Promise<Workflow[]>
  getSessions(): Promise<Session[]>
  getSettings(): Promise<AppSettings>
  setSettings(patch: Partial<AppSettings>): Promise<AppSettings>
  readFile(path: string): Promise<string>
  listSessionFiles(sessionId: string): Promise<SessionFilesDTO>
  readTracking(sessionId: string): Promise<string | null>
  getInstallStatus(): Promise<InstallStatusDTO>
  runInstall(): Promise<InstallStatusDTO>
  runUninstall(): Promise<InstallStatusDTO>
  discardSession(sessionId: string): Promise<void>
  unarchiveSession(sessionId: string): Promise<void>
  getSessionSummary(sessionId: string): Promise<SessionSummary | null>
  pickDirectory(defaultPath?: string): Promise<string | null>
  launchWorkflow(req: LaunchRequest): Promise<LaunchResult>
  focusTerminal(req: FocusTerminalRequest): Promise<FocusTerminalResult>
  ptyList(): Promise<PtyInfo[]>
  ptySpawn(req: SpawnInAppRequest): Promise<PtyInfo>
  ptyWrite(id: string, data: string): Promise<void>
  ptyResize(id: string, cols: number, rows: number): Promise<void>
  ptyKill(id: string): Promise<void>
  ptyBuffer(id: string): Promise<string>
  onPtyData(cb: (e: { id: string; data: string }) => void): () => void
  onPtyExit(cb: (e: { id: string; exitCode: number }) => void): () => void
  onPtyListUpdate(cb: (list: PtyInfo[]) => void): () => void

  onUniverses(cb: (u: Universe[]) => void): () => void
  onWorkflows(cb: (w: Workflow[]) => void): () => void
  onSessions(cb: (s: Session[]) => void): () => void
  onFileChanged(cb: (path: string) => void): () => void
}

declare global {
  interface Window {
    rcc: RccApi
  }
}
