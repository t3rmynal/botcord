import { useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { api, type InboxRow } from '../lib/ipc'

export function Inbox() {
  const [list, setList] = useState<InboxRow[]>([])
  const [adding, setAdding] = useState(false)
  const [name, setName] = useState('')
  const [url, setUrl] = useState('https://mail.proton.me/')
  const [domain, setDomain] = useState('')
  const [busy, setBusy] = useState(false)

  const refresh = async () => {
    const l = await api.inboxesList()
    setList(l)
  }

  useEffect(() => {
    refresh().catch(() => {})
  }, [])

  const add = async () => {
    if (!name.trim()) return
    setBusy(true)
    try {
      await api.inboxesAdd(
        name.trim(),
        url.trim() || 'https://mail.proton.me/',
        domain.trim() || null,
      )
      setName('')
      setUrl('https://mail.proton.me/')
      setDomain('')
      setAdding(false)
      await refresh()
    } finally {
      setBusy(false)
    }
  }

  const remove = async (id: string) => {
    await api.inboxesRemove(id)
    setList(list.filter((i) => i.id !== id))
  }

  return (
    <div className="h-full overflow-auto">
      <div className="p-6 flex flex-col gap-4">
        <div className="flex items-center gap-2">
          <div className="text-[11px] uppercase tracking-[0.22em] text-ink-2">
            {list.length} inbox{list.length === 1 ? '' : 'es'}
          </div>
          <button
            onClick={() => setAdding((v) => !v)}
            className="ml-auto h-7 px-3 text-[10px] uppercase tracking-[0.22em] border border-line-2 hover:border-ink transition"
          >
            {adding ? 'cancel' : '+ add inbox'}
          </button>
        </div>

        <AnimatePresence>
          {adding && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className="overflow-hidden"
            >
              <div className="border border-line bg-bg-2/60 p-4 flex flex-col gap-3">
                <label className="flex flex-col gap-1">
                  <span className="label">name</span>
                  <input
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="my proton / gmail / work"
                    className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px]"
                  />
                </label>
                <label className="flex flex-col gap-1">
                  <span className="label">url</span>
                  <input
                    value={url}
                    onChange={(e) => setUrl(e.target.value)}
                    placeholder="https://mail.proton.me/"
                    className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                  />
                </label>
                <label className="flex flex-col gap-1">
                  <span className="label">mail domain (optional, for register auto-gen)</span>
                  <input
                    value={domain}
                    onChange={(e) => setDomain(e.target.value)}
                    placeholder="t3rmynal.xyz"
                    className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                  />
                </label>
                <button
                  onClick={add}
                  disabled={busy || !name.trim()}
                  className="h-9 self-start px-5 border border-line-3 hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.22em] disabled:opacity-30 transition"
                >
                  {busy ? 'adding...' : 'add'}
                </button>
                <div className="label">
                  cookies are stored per inbox, login once per inbox. nothing shared between them.
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {list.length === 0 && !adding ? (
          <div className="h-[160px] border border-dashed border-line-2 grid place-items-center text-ink-3 text-[11px] uppercase tracking-[0.2em]">
            // no inboxes yet
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-3">
            <AnimatePresence initial={false}>
              {list.map((i) => (
                <motion.div
                  key={i.id}
                  layout
                  initial={{ opacity: 0, y: 4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, x: -12 }}
                  className="border border-line bg-bg-2/60 p-4 flex flex-col gap-2"
                >
                  <div className="text-[13px] font-medium truncate">{i.name}</div>
                  <div className="text-[10px] text-ink-3 font-mono truncate">{i.url}</div>
                  {i.domain && (
                    <div className="text-[10px] text-ink-2 font-mono truncate">
                      @{i.domain}
                    </div>
                  )}
                  <div className="flex items-center gap-1 pt-2">
                    <button
                      onClick={() => api.inboxesOpen(i.id).catch(() => {})}
                      className="h-7 px-3 border border-line-3 hover:border-ink text-[10px] uppercase tracking-[0.22em] transition"
                    >
                      open
                    </button>
                    <button
                      onClick={() => remove(i.id)}
                      className="h-7 px-3 border border-transparent hover:border-bad text-[10px] uppercase tracking-[0.22em] text-bad/80 hover:text-bad transition ml-auto"
                    >
                      del
                    </button>
                  </div>
                </motion.div>
              ))}
            </AnimatePresence>
          </div>
        )}
      </div>
    </div>
  )
}
