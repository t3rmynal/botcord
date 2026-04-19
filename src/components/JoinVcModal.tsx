import { useEffect, useMemo, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { clsx } from 'clsx'
import { api, type JoinResult, type VoiceMode } from '../lib/ipc'
import { useServers } from '../store/servers'

export function JoinVcModal({
  open,
  onClose,
  accountIds,
}: {
  open: boolean
  onClose: () => void
  accountIds: string[]
}) {
  const channels = useServers((s) => s.channels)
  const guilds = useServers((s) => s.guilds)
  const refresh = useServers((s) => s.refresh)

  const [cid, setCid] = useState('')
  const [gid, setGid] = useState('')
  const [mode, setMode] = useState<VoiceMode>('normal')
  const [joining, setJoining] = useState(false)
  const [result, setResult] = useState<JoinResult[] | null>(null)
  const [status, setStatus] = useState<Record<string, string>>({})

  useEffect(() => {
    if (open) refresh().catch(() => {})
  }, [open, refresh])

  useEffect(() => {
    if (!open) {
      setResult(null)
      setJoining(false)
      setStatus({})
    }
  }, [open])

  useEffect(() => {
    if (!open) return
    let un: UnlistenFn | null = null
    listen<{ account_id: string; state: string }>('voice:status', (e) => {
      setStatus((prev) => ({ ...prev, [e.payload.account_id]: e.payload.state }))
    }).then((u) => (un = u))
    return () => {
      if (un) un()
    }
  }, [open])

  const pickedChannel = channels.find((c) => c.channel_id === cid)
  const pickedGuild = useMemo(() => {
    if (pickedChannel?.guild_id) return pickedChannel.guild_id
    return gid.trim() || undefined
  }, [pickedChannel, gid])

  const onJoin = async () => {
    if (!cid.trim() || accountIds.length === 0) return
    setJoining(true)
    try {
      const res = await api.voiceBulkJoin(accountIds, cid.trim(), mode, pickedGuild || undefined)
      setResult(res)
    } finally {
      setJoining(false)
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
            className="w-[540px] max-w-full border border-line-3 bg-bg overflow-hidden"
          >
            <div className="h-10 px-4 flex items-center gap-3 border-b border-line">
              <span className="text-[11px] uppercase tracking-[0.22em]">join voice</span>
              <button
                className="ml-auto h-7 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-3 hover:text-ink transition"
                onClick={onClose}
              >
                esc
              </button>
            </div>
            <div className="p-4 flex flex-col gap-3">
              <div className="flex flex-col gap-1.5">
                <div className="label">channel</div>
                <select
                  value={channels.find((c) => c.channel_id === cid) ? cid : ''}
                  onChange={(e) => setCid(e.target.value)}
                  className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px]"
                >
                  <option value="">custom id...</option>
                  {channels.map((c) => {
                    const g = guilds.find((x) => x.guild_id === c.guild_id)
                    return (
                      <option key={c.channel_id} value={c.channel_id}>
                        {c.favorite ? '★ ' : ''}
                        {g?.name ? `${g.name} / ` : ''}
                        {c.name || c.channel_id.slice(0, 12)}
                      </option>
                    )
                  })}
                </select>
                <input
                  value={cid}
                  onChange={(e) => setCid(e.target.value)}
                  placeholder="channel id"
                  className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                />
                {!pickedChannel && (
                  <input
                    value={gid}
                    onChange={(e) => setGid(e.target.value)}
                    placeholder="guild id"
                    className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                  />
                )}
              </div>

              <div className="flex flex-col gap-1.5">
                <div className="label">mode</div>
                <div className="grid grid-cols-2 gap-2">
                  <ModeBtn
                    active={mode === 'normal'}
                    onClick={() => setMode('normal')}
                    title="normal"
                    hint="just present"
                  />
                  <ModeBtn
                    active={mode === 'silent'}
                    onClick={() => setMode('silent')}
                    title="silent"
                    hint="mute + deafen"
                  />
                </div>
              </div>

              <div className="flex items-center justify-between pt-1">
                <div className="text-[10px] uppercase tracking-[0.22em] text-ink-3">
                  {String(accountIds.length).padStart(2, '0')} selected
                </div>
                <button
                  onClick={onJoin}
                  disabled={joining || !cid.trim() || accountIds.length === 0}
                  className="h-8 px-5 border border-line-3 hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.22em] disabled:opacity-30 transition"
                >
                  {joining ? 'joining...' : '→ join'}
                </button>
              </div>

              {joining && Object.keys(status).length > 0 && (
                <div className="border border-line max-h-[120px] overflow-auto bg-bg-2/50">
                  {Object.entries(status).map(([aid, st]) => (
                    <div
                      key={aid}
                      className="h-7 px-3 flex items-center gap-2 border-b border-line/60 last:border-b-0 text-[11px]"
                    >
                      <span className="w-1.5 h-1.5 bg-ink-3 animate-pulse" />
                      <span className="font-mono text-ink-3">{aid.slice(0, 8)}</span>
                      <span className="ml-auto text-[10px] uppercase tracking-[0.18em] text-ink-2">
                        {st}
                      </span>
                    </div>
                  ))}
                </div>
              )}

              {result && (
                <div className="border border-line max-h-[200px] overflow-auto">
                  {result.map((r) => (
                    <div
                      key={r.account_id}
                      className="h-8 px-3 flex items-center gap-2 border-b border-line/60 last:border-b-0 text-[11px]"
                    >
                      <span
                        className={clsx(
                          'w-1.5 h-1.5',
                          r.ok ? 'bg-ok' : 'bg-bad',
                        )}
                      />
                      <span className="font-mono text-ink-3">{r.account_id.slice(0, 8)}</span>
                      <span className={clsx('ml-auto truncate uppercase tracking-[0.18em] text-[10px]', r.ok ? 'text-ok' : 'text-bad')}>
                        {r.ok ? 'joined' : r.error}
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

function ModeBtn({
  active,
  onClick,
  title,
  hint,
}: {
  active: boolean
  onClick: () => void
  title: string
  hint: string
}) {
  return (
    <button
      onClick={onClick}
      className={clsx(
        'h-14 border flex flex-col items-start justify-center px-3 gap-0.5 transition',
        active ? 'border-ink bg-bg-3 text-ink' : 'border-line-2 bg-bg-2 text-ink-3 hover:text-ink hover:border-line-3',
      )}
    >
      <span className="text-[12px] uppercase tracking-[0.22em]">{title}</span>
      <span className="text-[10px] text-ink-3">{hint}</span>
    </button>
  )
}
