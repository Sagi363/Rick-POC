import { contextBridge, ipcRenderer } from 'electron'
import { IPC, type RccApi } from '@shared/ipc'

const subscribe = (channel: string, cb: (payload: any) => void): (() => void) => {
  const listener = (_e: Electron.IpcRendererEvent, payload: any): void => cb(payload)
  ipcRenderer.on(channel, listener)
  return () => ipcRenderer.off(channel, listener)
}

const api: RccApi = {
  getUniverses: () => ipcRenderer.invoke(IPC.GetUniverses),
  getWorkflows: (universe) => ipcRenderer.invoke(IPC.GetWorkflows, universe),
  getSessions: () => ipcRenderer.invoke(IPC.GetSessions),
  getSettings: () => ipcRenderer.invoke(IPC.GetSettings),
  setSettings: (patch) => ipcRenderer.invoke(IPC.SetSettings, patch),
  readFile: (path) => ipcRenderer.invoke(IPC.ReadFile, path),
  listSessionFiles: (sessionId) => ipcRenderer.invoke(IPC.ListSessionFiles, sessionId),
  readTracking: (sessionId) => ipcRenderer.invoke(IPC.ReadTracking, sessionId),
  getInstallStatus: () => ipcRenderer.invoke(IPC.GetInstallStatus),
  runInstall: () => ipcRenderer.invoke(IPC.RunInstall),
  runUninstall: () => ipcRenderer.invoke(IPC.RunUninstall),
  discardSession: (id) => ipcRenderer.invoke(IPC.DiscardSession, id),
  unarchiveSession: (id) => ipcRenderer.invoke(IPC.UnarchiveSession, id),
  getSessionSummary: (id) => ipcRenderer.invoke(IPC.GetSessionSummary, id),
  pickDirectory: (defaultPath) => ipcRenderer.invoke(IPC.PickDirectory, defaultPath),
  launchWorkflow: (req) => ipcRenderer.invoke(IPC.LaunchWorkflow, req),
  focusTerminal: (req) => ipcRenderer.invoke(IPC.FocusTerminal, req),
  ptyList: () => ipcRenderer.invoke(IPC.PtyList),
  ptySpawn: (req) => ipcRenderer.invoke(IPC.PtySpawn, req),
  ptyWrite: (id, data) => ipcRenderer.invoke(IPC.PtyWrite, id, data),
  ptyResize: (id, cols, rows) => ipcRenderer.invoke(IPC.PtyResize, id, cols, rows),
  ptyKill: (id) => ipcRenderer.invoke(IPC.PtyKill, id),
  ptyBuffer: (id) => ipcRenderer.invoke(IPC.PtyBuffer, id),
  onPtyData: (cb) => subscribe(IPC.PtyData, cb),
  onPtyExit: (cb) => subscribe(IPC.PtyExit, cb),
  onPtyListUpdate: (cb) => subscribe(IPC.PtyListUpdate, cb),

  onUniverses: (cb) => subscribe(IPC.UniversesUpdate, cb),
  onWorkflows: (cb) => subscribe(IPC.WorkflowsUpdate, cb),
  onSessions: (cb) => subscribe(IPC.SessionsUpdate, cb),
  onFileChanged: (cb) => subscribe(IPC.FileChanged, cb)
}

contextBridge.exposeInMainWorld('rcc', api)
