import { useEffect, useRef, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { clsx } from 'clsx'
import { api, type BroadcastProgress, type BroadcastResult } from '../lib/ipc'

type Mode = 'dms' | 'servers'

export function BroadcastModal({
  open,
  onClose,
  accountIds,
}: {
  open: boolean
  onClose: () => void
  accountIds: string[]
}) {
  const [mode, setMode] = useState<Mode>('dms')
  const [text, setText] = useState('')
  const [imagePath, setImagePath] = useState<string | null>(null)
  const [skipGroups, setSkipGroups] = useState(true)
  const [skipAnnouncements, setSkipAnnouncements] = useState(true)
  const [perGuildLimit, setPerGuildLimit] = useState(1)
  const [minDelay, setMinDelay] = useState(900)
  const [maxDelay, setMaxDelay] = useState(2600)
  const [running, setRunning] = useState(false)
  const [progress, setProgress] = useState<BroadcastProgress[]>([])
  const [result, setResult] = useState<BroadcastResult | null>(null)
  const listRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    let un: UnlistenFn | null = null
    listen<BroadcastProgress>('broadcast:progress', (e) => {
      setProgress((p) => [...p, e.payload])
      requestAnimationFrame(() => {
        const n = listRef.current
        if (n) n.scrollTop = n.scrollHeight
      })
    }).then((u) => (un = u))
    return () => {
      if (un) un()
    }
  }, [open])

  useEffect(() => {
    if (!open) {
      setProgress([])
      setResult(null)
      setRunning(false)
    }
  }, [open])

  useEffect(() => {
    if (mode === 'servers') {
      setMinDelay((v) => Math.max(v, 2000))
      setMaxDelay((v) => Math.max(v, 5500))
    }
  }, [mode])

  const pickImage = async () => {
    const path = await openDialog({
      multiple: false,
      filters: [{ name: 'image', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
    })
    if (typeof path === 'string') setImagePath(path)
  }

  const fire = async () => {
    if (running) return
    if (accountIds.length === 0) return
    if (!text.trim() && !imagePath) return
    setRunning(true)
    setProgress([])
    setResult(null)
    try {
      if (mode === 'dms') {
        const r = await api.broadcastDms({
          account_ids: accountIds,
          text,
          image_path: imagePath,
          skip_groups: skipGroups,
          min_delay_ms: minDelay,
          max_delay_ms: maxDelay,
        })
        setResult(r)
      } else {
        const r = await api.broadcastGuilds({
          account_ids: accountIds,
          text,
          image_path: imagePath,
          per_guild_limit: perGuildLimit,
          skip_announcements: skipAnnouncements,
          min_delay_ms: minDelay,
          max_delay_ms: maxDelay,
        })
        setResult(r)
      }
    } catch (e) {
      setProgress((p) => [
        ...p,
        { account_id: '?', channel_id: '?', recipient: null, ok: false, error: String(e) },
      ])
    } finally {
      setRunning(false)
    }
  }

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="fixed inset-0 bg-black/75 backdrop-blur-sm grid place-items-center z-50 p-6"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          onClick={onClose}
        >
          <motion.div
            onClick={(e) => e.stopPropagation()}
            initial={{ y: 16, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: 8, opacity: 0 }}
            transition={{ type: 'spring', stiffness: 300, damping: 28 }}
            className="w-[640px] max-w-full border border-line-3 bg-bg overflow-hidden flex flex-col"
          >
            <div className="h-10 px-4 flex items-center gap-3 border-b border-line">
              <span className="text-[11px] uppercase tracking-[0.22em]">broadcast</span>
              <div className="flex border border-line-2">
                <TabBtn active={mode === 'dms'} onClick={() => setMode('dms')}>
                  dms
                </TabBtn>
                <TabBtn active={mode === 'servers'} onClick={() => setMode('servers')}>
                  servers
                </TabBtn>
              </div>
              <span className="ml-auto text-[10px] text-ink-3">
                {String(accountIds.length).padStart(2, '0')} account
                {accountIds.length === 1 ? '' : 's'}
              </span>
              <button
                className="h-7 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-3 hover:text-ink transition"
                onClick={onClose}
              >
                esc
              </button>
            </div>

            <div className="p-4 flex flex-col gap-3">
              <div className="flex flex-col gap-1.5">
                <div className="label">message</div>
                <textarea
                  value={text}
                  onChange={(e) => setText(e.target.value)}
                  placeholder="> yo, new account: @handle"
                  spellCheck={false}
                  className="min-h-[100px] max-h-[220px] p-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono resize-y"
                />
              </div>

              <div className="grid grid-cols-[1fr_auto] gap-2 items-end">
                <div className="flex flex-col gap-1.5">
                  <div className="label">image (optional)</div>
                  <div className="h-9 px-3 bg-bg-2 border border-line-2 flex items-center text-[11px] text-ink-3 font-mono truncate">
                    {imagePath || '—'}
                  </div>
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={pickImage}
                    className="h-9 px-3 border border-line-3 hover:border-ink text-[11px] uppercase tracking-[0.22em] transition"
                  >
                    pick
                  </button>
                  {imagePath && (
                    <button
                      onClick={() => setImagePath(null)}
                      className="h-9 px-3 border border-line hover:border-bad text-[11px] uppercase tracking-[0.22em] text-bad/80 hover:text-bad transition"
                    >
                      ×
                    </button>
                  )}
                </div>
              </div>

              {mode === 'dms' ? (
                <div className="grid grid-cols-[auto_1fr_1fr] gap-3 items-end">
                  <label className="flex items-center gap-2 cursor-pointer h-9">
                    <input
                      type="checkbox"
                      checked={skipGroups}
                      onChange={(e) => setSkipGroups(e.target.checked)}
                      className="accent-white"
                    />
                    <span className="text-[10px] uppercase tracking-[0.22em] text-ink-2">
                      skip groups
                    </span>
                  </label>
                  <NumInput label="min delay ms" value={minDelay} onChange={setMinDelay} />
                  <NumInput label="max delay ms" value={maxDelay} onChange={setMaxDelay} />
                </div>
              ) : (
                <div className="grid grid-cols-[auto_auto_1fr_1fr] gap-3 items-end">
                  <NumInput
                    label="ch / guild"
                    value={perGuildLimit}
                    onChange={(v) => setPerGuildLimit(Math.max(1, Math.min(v, 10)))}
                  />
                  <label className="flex items-center gap-2 cursor-pointer h-9">
                    <input
                      type="checkbox"
                      checked={skipAnnouncements}
                      onChange={(e) => setSkipAnnouncements(e.target.checked)}
                      className="accent-white"
                    />
                    <span className="text-[10px] uppercase tracking-[0.22em] text-ink-2">
                      skip announcements
                    </span>
                  </label>
                  <NumInput label="min delay ms" value={minDelay} onChange={setMinDelay} />
                  <NumInput label="max delay ms" value={maxDelay} onChange={setMaxDelay} />
                </div>
              )}

              {mode === 'servers' && (
                <div className="flex items-start gap-2 p-2 border border-warn/30 text-[11px] text-warn bg-warn/5">
                  <span>!</span>
                  <div className="flex flex-col gap-0.5">
                    <div className="uppercase tracking-[0.18em] text-[10px]">heads up</div>
                    <div className="text-ink-3">
                      blasting in every server is detectable. keep `ch / guild` low, delays high.
                    </div>
                  </div>
                </div>
              )}

              <div className="flex items-center justify-between pt-1">
                <div className="text-[10px] uppercase tracking-[0.22em] text-ink-3">
                  {running
                    ? `sending... ${progress.length}`
                    : result
                    ? `ok ${result.delivered} / fail ${result.failed} / skip ${result.skipped}`
                    : 'ready'}
                </div>
                <button
                  onClick={fire}
                  disabled={running || accountIds.length === 0 || (!text.trim() && !imagePath)}
                  className="h-9 px-5 border border-line-3 hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.22em] disabled:opacity-30 transition"
                >
                  {running ? 'sending...' : '→ broadcast'}
                </button>
              </div>

              {(progress.length > 0 || running) && (
                <div
                  ref={listRef}
                  className="border border-line max-h-[200px] overflow-auto bg-bg-2/50"
                >
                  {progress.map((r, i) => (
                    <div
                      key={i}
                      className="h-7 px-3 flex items-center gap-2 border-b border-line/60 last:border-b-0 text-[11px]"
                    >
                      <span className={clsx('w-1.5 h-1.5', r.ok ? 'bg-ok' : 'bg-bad')} />
                      <span className="font-mono text-ink-3">{r.account_id.slice(0, 6)}</span>
                      <span className="text-ink-3">·</span>
                      <span className="truncate text-ink-2">
                        {r.recipient || r.channel_id.slice(0, 10)}
                      </span>
                      <span
                        className={clsx(
                          'ml-auto text-[10px] uppercase tracking-[0.18em]',
                          r.ok ? 'text-ok' : 'text-bad',
                        )}
                      >
                        {r.ok ? 'sent' : r.error?.slice(0, 40) || 'fail'}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}

function TabBtn({
  active,
  onClick,
  children,
}: {
  active: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      onClick={onClick}
      className={clsx(
        'h-7 px-3 text-[10px] uppercase tracking-[0.22em] transition',
        active ? 'bg-bg-3 text-ink' : 'text-ink-3 hover:text-ink',
      )}
    >
      {children}
    </button>
  )
}

function NumInput({
  label,
  value,
  onChange,
}: {
  label: string
  value: number
  onChange: (v: number) => void
}) {
  return (
    <label className="flex flex-col gap-1">
      <span className="label">{label}</span>
      <input
        type="number"
        min={0}
        value={value}
        onChange={(e) => onChange(Number(e.target.value) || 0)}
        className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
      />
    </label>
  )
}
