import { useEffect, useState, useCallback } from 'react'
import type { AppSettings, PtyInfo, Session, Universe, Workflow } from '@shared/types'

const api = window.rcc

export function useAppState(): AppState {
  const [universes, setUniverses] = useState<Universe[]>([])
  const [workflows, setWorkflows] = useState<Workflow[]>([])
  const [sessions, setSessions] = useState<Session[]>([])
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null)
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [ptys, setPtys] = useState<PtyInfo[]>([])

  useEffect(() => {
    void Promise.all([
      api.getUniverses(),
      api.getWorkflows(),
      api.getSessions(),
      api.getSettings(),
      api.ptyList()
    ]).then(([u, w, s, st, p]) => {
      setUniverses(u)
      setWorkflows(w)
      setSessions(s)
      setSettings(st)
      setPtys(p)
    })

    const offU = api.onUniverses(setUniverses)
    const offW = api.onWorkflows(setWorkflows)
    const offS = api.onSessions(setSessions)
    const offP = api.onPtyListUpdate(setPtys)
    return () => {
      offU()
      offW()
      offS()
      offP()
    }
  }, [])

  const setLastUniverse = useCallback(async (name?: string) => {
    const next = await api.setSettings({ lastUniverse: name })
    setSettings(next)
    setSessions(await api.getSessions())
  }, [])

  const patchSettings = useCallback(async (patch: Partial<AppSettings>) => {
    const next = await api.setSettings(patch)
    setSettings(next)
  }, [])

  return {
    universes,
    workflows,
    sessions,
    settings,
    selectedSessionId,
    setSelectedSessionId,
    selectedFile,
    setSelectedFile,
    setLastUniverse,
    patchSettings,
    ptys
  }
}

export interface AppState {
  universes: Universe[]
  workflows: Workflow[]
  sessions: Session[]
  settings: AppSettings | null
  selectedSessionId: string | null
  setSelectedSessionId: (id: string | null) => void
  selectedFile: string | null
  setSelectedFile: (path: string | null) => void
  setLastUniverse: (name?: string) => Promise<void>
  patchSettings: (patch: Partial<AppSettings>) => Promise<void>
  ptys: PtyInfo[]
}
