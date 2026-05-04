import { useCallback, useEffect, useRef, useState } from 'react'
import clsx from 'clsx'

type Axis = 'x' | 'y'

interface UsePersistedSize {
  size: number
  onMouseDown: (e: React.MouseEvent) => void
}

export function usePersistedSize(
  key: string,
  initial: number,
  min: number,
  max: number,
  axis: Axis
): UsePersistedSize {
  const [size, setSize] = useState<number>(() => {
    if (typeof window === 'undefined') return initial
    const stored = window.localStorage.getItem(`rcc:size:${key}`)
    const parsed = stored ? Number(stored) : NaN
    return Number.isFinite(parsed) ? clamp(parsed, min, max) : initial
  })
  const startRef = useRef<{ pos: number; size: number } | null>(null)

  useEffect(() => {
    window.localStorage.setItem(`rcc:size:${key}`, String(size))
  }, [key, size])

  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      startRef.current = {
        pos: axis === 'x' ? e.clientX : e.clientY,
        size
      }
      const move = (ev: MouseEvent): void => {
        if (!startRef.current) return
        const cur = axis === 'x' ? ev.clientX : ev.clientY
        const delta = cur - startRef.current.pos
        setSize(clamp(startRef.current.size + delta, min, max))
      }
      const up = (): void => {
        startRef.current = null
        window.removeEventListener('mousemove', move)
        window.removeEventListener('mouseup', up)
        document.body.style.cursor = ''
        document.body.style.userSelect = ''
      }
      window.addEventListener('mousemove', move)
      window.addEventListener('mouseup', up)
      document.body.style.cursor = axis === 'x' ? 'col-resize' : 'row-resize'
      document.body.style.userSelect = 'none'
    },
    [axis, min, max, size]
  )

  return { size, onMouseDown }
}

interface ResizerProps {
  axis: Axis
  onMouseDown: (e: React.MouseEvent) => void
}

export function Resizer({ axis, onMouseDown }: ResizerProps): JSX.Element {
  return (
    <div
      role="separator"
      onMouseDown={onMouseDown}
      className={clsx(
        'group relative shrink-0 bg-zinc-800 transition-colors hover:bg-emerald-600',
        axis === 'x' ? 'w-px cursor-col-resize' : 'h-[2px] cursor-row-resize'
      )}
    >
      <div
        className={clsx(
          'absolute',
          axis === 'x' ? '-left-1 top-0 h-full w-2' : 'left-0 -top-1 h-3 w-full'
        )}
      />
    </div>
  )
}

function clamp(n: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, n))
}
