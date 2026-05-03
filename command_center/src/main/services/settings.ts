import { mkdir, readFile, writeFile, rename } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { app } from 'electron'
import { DEFAULT_SETTINGS, type AppSettings } from '@shared/types'

const FILE_NAME = 'settings.json'

export class SettingsService {
  private state: AppSettings = { ...DEFAULT_SETTINGS }
  private path: string

  constructor() {
    this.path = join(app.getPath('userData'), FILE_NAME)
  }

  async load(): Promise<AppSettings> {
    try {
      const raw = await readFile(this.path, 'utf8')
      const parsed = JSON.parse(raw) as Partial<AppSettings>
      this.state = { ...DEFAULT_SETTINGS, ...parsed }
    } catch {
      this.state = { ...DEFAULT_SETTINGS }
    }
    return this.state
  }

  get(): AppSettings {
    return this.state
  }

  async patch(p: Partial<AppSettings>): Promise<AppSettings> {
    this.state = { ...this.state, ...p }
    await this.persist()
    return this.state
  }

  private async persist(): Promise<void> {
    await mkdir(dirname(this.path), { recursive: true })
    const tmp = `${this.path}.tmp`
    await writeFile(tmp, JSON.stringify(this.state, null, 2), 'utf8')
    await rename(tmp, this.path)
  }
}
