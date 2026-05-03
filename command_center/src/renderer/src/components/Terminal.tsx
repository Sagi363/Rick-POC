import { useEffect, useRef } from 'react'
import { Terminal as XTerm } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'

interface Props {
  ptyId: string
  active: boolean
}

export function Terminal({ ptyId, active }: Props): JSX.Element {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const xtermRef = useRef<XTerm | null>(null)
  const fitRef = useRef<FitAddon | null>(null)

  useEffect(() => {
    const host = containerRef.current
    if (!host) return

    const term = new XTerm({
      theme: {
        background: '#09090b',
        foreground: '#e4e4e7',
        cursor: '#34d399',
        selectionBackground: '#27272a'
      },
      cursorBlink: true,
      cursorStyle: 'bar',
      fontFamily: '"SF Mono", Menlo, Monaco, Consolas, monospace',
      fontSize: 12,
      lineHeight: 1.2,
      scrollback: 5000,
      allowProposedApi: true
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(host)
    try {
      fit.fit()
    } catch {
      // ignore — first fit can fail before layout
    }
    xtermRef.current = term
    fitRef.current = fit

    let disposed = false

    void window.rcc.ptyBuffer(ptyId).then((buf) => {
      if (!disposed && buf) term.write(buf)
    })

    const offData = window.rcc.onPtyData((e) => {
      if (e.id === ptyId) term.write(e.data)
    })

    const inputDisposable = term.onData((d) => {
      void window.rcc.ptyWrite(ptyId, d)
    })

    const sendResize = (): void => {
      try {
        fit.fit()
        void window.rcc.ptyResize(ptyId, term.cols, term.rows)
      } catch {
        // ignore
      }
    }

    const ro = new ResizeObserver(sendResize)
    ro.observe(host)
    sendResize()

    return () => {
      disposed = true
      ro.disconnect()
      offData()
      inputDisposable.dispose()
      term.dispose()
      xtermRef.current = null
      fitRef.current = null
    }
  }, [ptyId])

  useEffect(() => {
    if (active) {
      xtermRef.current?.focus()
      try {
        fitRef.current?.fit()
      } catch {
        // ignore
      }
    }
  }, [active])

  return <div ref={containerRef} className="h-full w-full" />
}
