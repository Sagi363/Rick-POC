import { useEffect, useState } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

interface Props {
  path: string | null
}

export function FilePreview({ path }: Props): JSX.Element {
  const [content, setContent] = useState<string>('')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!path) {
      setContent('')
      setError(null)
      return
    }
    let cancelled = false
    setError(null)
    void window.rcc
      .readFile(path)
      .then((text) => {
        if (!cancelled) setContent(text)
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message)
      })

    const off = window.rcc.onFileChanged((p) => {
      if (p === path) void window.rcc.readFile(path).then((t) => !cancelled && setContent(t))
    })
    return () => {
      cancelled = true
      off()
    }
  }, [path])

  if (!path) {
    return (
      <Pane title="Preview">
        <Centered>Select a file to preview.</Centered>
      </Pane>
    )
  }

  if (error) {
    return (
      <Pane title={path.split('/').pop()!}>
        <Centered>
          <span className="text-rose-400">Read failed: {error}</span>
        </Centered>
      </Pane>
    )
  }

  const ext = path.split('.').pop()?.toLowerCase()
  return (
    <Pane title={path.split('/').pop()!}>
      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-3 text-sm">
        {ext === 'md' || ext === 'markdown' ? (
          <article className="markdown">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{content}</ReactMarkdown>
          </article>
        ) : (
          <pre className="whitespace-pre-wrap break-words font-mono text-xs text-zinc-300">
            {content}
          </pre>
        )}
      </div>
    </Pane>
  )
}

function Pane({ title, children }: { title: string; children: React.ReactNode }): JSX.Element {
  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <div className="shrink-0 border-b border-zinc-800 px-3 py-2 text-xs uppercase tracking-wider text-zinc-500">
        {title}
      </div>
      {children}
    </section>
  )
}

function Centered({ children }: { children: React.ReactNode }): JSX.Element {
  return (
    <div className="flex min-h-0 flex-1 items-center justify-center text-xs text-zinc-600">
      {children}
    </div>
  )
}
