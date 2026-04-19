import { useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { useAccounts } from '../store/accounts'
import { clsx } from 'clsx'

export function TokenCheckCard() {
  const [raw, setRaw] = useState('')
  const [open, setOpen] = useState(true)
  const checking = useAccounts((s) => s.checking)
  const adding = useAccounts((s) => s.adding)
  const lastCheck = useAccounts((s) => s.lastCheck)
  const lastTokens = useAccounts((s) => s.lastTokens)
  const check = useAccounts((s) => s.check)
  const addValid = useAccounts((s) => s.addValid)
  const clearLastCheck = useAccounts((s) => s.clearLastCheck)

  const onCheck = () => {
    if (!checking) check(raw)
  }
  const onAdd = async () => {
    await addValid()
    setRaw('')
  }

  const valid: { token: string; premium: number | null }[] = []
  const invalid: string[] = []
  if (lastCheck && lastTokens) {
    lastCheck.forEach((item, i) => {
      const tok = lastTokens[i]
      if (!tok) return
      if (item.ok) valid.push({ token: tok, premium: item.premium_type })
      else invalid.push(tok)
    })
  }

  return (
    <div className="border border-line bg-bg/60 backdrop-blur-sm">
      <button
        className="w-full h-10 px-4 flex items-center gap-3 text-[11px] uppercase tracking-[0.2em] text-ink-2 hover:text-ink transition"
        onClick={() => setOpen((v) => !v)}
      >
        <span className={clsx('inline-block w-2', open ? 'text-ink' : 'text-ink-3')}>
          {open ? '▾' : '▸'}
        </span>
        enter tokens
        <span className="ml-auto text-[10px] text-ink-3">one per line</span>
      </button>

      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            key="body"
            initial={{ height: 0 }}
            animate={{ height: 'auto' }}
            exit={{ height: 0 }}
            transition={{ duration: 0.18 }}
            className="overflow-hidden border-t border-line"
          >
            <div className="p-4 flex flex-col gap-3">
              <textarea
                value={raw}
                onChange={(e) => setRaw(e.target.value)}
                spellCheck={false}
                placeholder="> paste tokens"
                onKeyDown={(e) => {
                  if (e.ctrlKey && e.key === 'Enter') onCheck()
                }}
                className="min-h-[120px] max-h-[240px] p-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono resize-y"
              />
              <div className="flex items-center gap-2">
                <button
                  disabled={checking || raw.trim().length === 0}
                  onClick={onCheck}
                  className="h-8 px-4 border border-line-3 hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.2em] disabled:opacity-30 transition"
                >
                  {checking ? 'checking...' : 'check'}
                </button>
                {lastCheck && (
                  <button
                    onClick={clearLastCheck}
                    className="h-8 px-3 text-[11px] uppercase tracking-[0.2em] text-ink-3 hover:text-ink transition"
                  >
                    clear
                  </button>
                )}
                <span className="ml-auto text-[10px] text-ink-4">ctrl+enter</span>
              </div>

              <AnimatePresence>
                {lastCheck && (
                  <motion.div
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -6 }}
                    transition={{ duration: 0.15 }}
                    className="grid grid-cols-2 gap-2"
                  >
                    <ResultColumn
                      kind="valid"
                      count={valid.length}
                      items={valid.map((v) => v.token)}
                      hasNitro={valid.some((v) => (v.premium ?? 0) > 0)}
                    />
                    <ResultColumn kind="invalid" count={invalid.length} items={invalid} />
                  </motion.div>
                )}
              </AnimatePresence>

              {lastCheck && valid.length > 0 && (
                <button
                  onClick={onAdd}
                  disabled={adding}
                  className="h-8 self-start px-4 border border-line-3 hover:border-ink text-[11px] uppercase tracking-[0.2em] disabled:opacity-40 transition"
                >
                  {adding ? 'adding...' : `+ ${valid.length} to panel`}
                </button>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}

function ResultColumn({
  kind,
  count,
  items,
  hasNitro,
}: {
  kind: 'valid' | 'invalid'
  count: number
  items: string[]
  hasNitro?: boolean
}) {
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    if (items.length === 0) return
    await navigator.clipboard.writeText(items.join('\n'))
    setCopied(true)
    setTimeout(() => setCopied(false), 1200)
  }
  return (
    <div className="border border-line bg-bg-2/60 flex flex-col">
      <div className="h-8 px-3 flex items-center gap-3 border-b border-line">
        <span
          className={clsx(
            'text-[10px] uppercase tracking-[0.22em]',
            kind === 'valid' ? 'text-ok' : 'text-bad',
          )}
        >
          {kind}
        </span>
        <span className="text-[11px] text-ink-2">{count}</span>
        {hasNitro && (
          <span className="text-[9px] uppercase tracking-[0.2em] text-ink-2 border border-ink-3 px-1">
            nitro inside
          </span>
        )}
        <button
          disabled={items.length === 0}
          onClick={copy}
          className="ml-auto text-[10px] uppercase tracking-[0.2em] text-ink-3 hover:text-ink disabled:opacity-30 transition"
        >
          [{copied ? 'copied' : 'copy'}]
        </button>
      </div>
      <div className="p-2 max-h-[140px] overflow-auto text-[11px] font-mono text-ink-3 flex flex-col gap-0.5">
        {items.length === 0 && <div className="text-ink-4 px-1">—</div>}
        {items.map((t, i) => (
          <div key={i} className="truncate px-1">
            {mask(t)}
          </div>
        ))}
      </div>
    </div>
  )
}

function mask(t: string) {
  if (t.length < 14) return t
  return `${t.slice(0, 10)}...${t.slice(-6)}`
}
