import type { Universe } from '@shared/types'

interface Props {
  universes: Universe[]
  active?: string
  onChange: (name?: string) => void
  onOpenSettings: () => void
}

export function TopBar({ universes, active, onChange, onOpenSettings }: Props): JSX.Element {
  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-zinc-800 px-4 [-webkit-app-region:drag]">
      <div className="flex items-center gap-3 [-webkit-app-region:no-drag]">
        <span className="ml-16 text-sm font-medium tracking-wide text-zinc-300">
          Rick Command Center
        </span>
        <span className="text-zinc-600">·</span>
        <select
          className="rounded-md border border-zinc-700 bg-zinc-900 px-2 py-1 text-xs text-zinc-200 focus:border-zinc-500 focus:outline-none"
          value={active ?? ''}
          onChange={(e) => onChange(e.target.value || undefined)}
        >
          <option value="">All universes</option>
          {universes.map((u) => (
            <option key={u.name} value={u.name}>
              {u.name}
            </option>
          ))}
        </select>
      </div>
      <button
        onClick={onOpenSettings}
        className="rounded-md border border-zinc-700 px-2 py-1 text-xs text-zinc-300 hover:border-zinc-500 hover:text-zinc-100 [-webkit-app-region:no-drag]"
      >
        Settings
      </button>
    </header>
  )
}
