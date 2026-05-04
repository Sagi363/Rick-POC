import type { Workflow } from '@shared/types'

interface Props {
  workflow: Workflow
  onLaunch: () => void
}

export function WorkflowCard({ workflow, onLaunch }: Props): JSX.Element {
  return (
    <div className="rounded-md border border-zinc-800 bg-zinc-900 px-3 py-2">
      <div className="flex items-baseline justify-between gap-2">
        <span className="truncate text-sm font-medium text-zinc-100">{workflow.name}</span>
        <span className="shrink-0 truncate text-[10px] text-zinc-500">{workflow.universe}</span>
      </div>
      {workflow.description && (
        <p
          className="mt-1 line-clamp-2 text-[11px] leading-snug text-zinc-400"
          title={workflow.description}
        >
          {workflow.description}
        </p>
      )}
      {workflow.agents.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-1">
          {workflow.agents.slice(0, 6).map((a) => (
            <span
              key={a}
              className="rounded-full border border-zinc-800 bg-zinc-950 px-1.5 py-0.5 font-mono text-[10px] text-zinc-300"
            >
              {a}
            </span>
          ))}
          {workflow.agents.length > 6 && (
            <span className="text-[10px] text-zinc-500">+{workflow.agents.length - 6}</span>
          )}
        </div>
      )}
      <div className="mt-2 flex items-center gap-2 text-[10px] text-zinc-500">
        {workflow.params.length > 0 && (
          <span>
            {workflow.params.length} param{workflow.params.length === 1 ? '' : 's'}
          </span>
        )}
        {workflow.dependsOn.length > 0 && (
          <>
            {workflow.params.length > 0 && <span className="text-zinc-700">·</span>}
            <span>{workflow.dependsOn.length} dep</span>
          </>
        )}
        <button
          onClick={(e) => {
            e.stopPropagation()
            onLaunch()
          }}
          className="ml-auto rounded bg-emerald-700/40 px-2 py-0.5 text-[10px] text-emerald-200 hover:bg-emerald-700/70"
        >
          ▶ Launch
        </button>
      </div>
    </div>
  )
}
