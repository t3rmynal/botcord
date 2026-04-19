import { useEffect, useRef, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { clsx } from 'clsx'
import { api, type SocialResult, type SocialStep } from '../lib/ipc'

export function InviteModal({
  open,
  onClose,
  accountIds,
}: {
  open: boolean
  onClose: () => void
  accountIds: string[]
}) {
  const [invite, setInvite] = useState('')
  const [minDelay, setMinDelay] = useState(1200)
  const [maxDelay, setMaxDelay] = useState(3500)
  const [running, setRunning] = useState(false)
  const [steps, setSteps] = useState<SocialStep[]>([])
  const [result, setResult] = useState<SocialResult | null>(null)
  const listRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    let un: UnlistenFn | null = null
    listen<SocialStep>('invite:progress', (e) => {
      setSteps((p) => [...p, e.payload])
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
      setSteps([])
      setResult(null)
      setRunning(false)
    }
  }, [open])

  const fire = async () => {
    if (running || !invite.trim() || accountIds.length === 0) return
    setRunning(true)
    setSteps([])
    setResult(null)
    try {
      const r = await api.bulkJoinInvite({
        account_ids: accountIds,
        invite: invite.trim(),
        min_delay_ms: minDelay,
        max_delay_ms: maxDelay,
      })
      setResult(r)
    } catch (e) {
      setSteps((p) => [...p, { account_id: '?', state: 'error', message: String(e) }])
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
            className="w-[600px] max-w-full border border-line-3 bg-bg overflow-hidden"
          >
            <div className="h-10 px-4 flex items-center gap-3 border-b border-line">
              <span className="text-[11px] uppercase tracking-[0.22em]">join invite</span>
              <span className="ml-auto text-[10px] text-ink-3">
                {String(accountIds.length).padStart(2, '0')} accounts
              </span>
              <button
                className="h-7 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-3 hover:text-ink transition"
                onClick={onClose}
              >
                esc
              </button>
            </div>
            <div className="p-4 flex flex-col gap-3">
              <label className="flex flex-col gap-1">
                <span className="label">invite</span>
                <input
                  value={invite}
                  onChange={(e) => setInvite(e.target.value)}
                  placeholder="discord.gg/xxx or just the code"
                  className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                />
              </label>
              <div className="grid grid-cols-2 gap-3">
                <NumInput label="min delay ms" value={minDelay} onChange={setMinDelay} />
                <NumInput label="max delay ms" value={maxDelay} onChange={setMaxDelay} />
              </div>

              <div className="flex items-center justify-between pt-1">
                <div className="text-[10px] uppercase tracking-[0.22em] text-ink-3">
                  {running
                    ? `running... ${steps.length}`
                    : result
                    ? `ok ${result.ok} / fail ${result.failed}`
                    : 'ready'}
                </div>
                <button
                  onClick={fire}
                  disabled={running || accountIds.length === 0 || !invite.trim()}
                  className="h-9 min-w-[180px] px-5 border border-line-3 hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.22em] disabled:opacity-30 transition whitespace-nowrap"
                >
                  {running ? 'joining...' : '→ join'}
                </button>
              </div>

              <div className="label">
                if discord asks captcha, browser opens for each account that needs it. solve, app picks up automatically.
              </div>

              {(steps.length > 0 || running) && (
                <div
                  ref={listRef}
                  className="border border-line max-h-[220px] overflow-auto bg-bg-2/50"
                >
                  {steps.map((r, i) => (
                    <div
                      key={i}
                      className="h-7 px-3 flex items-center gap-2 border-b border-line/60 last:border-b-0 text-[11px]"
                    >
                      <span
                        className={clsx(
                          'w-1.5 h-1.5',
                          r.state === 'joined' ? 'bg-ok' : r.state === 'error' ? 'bg-bad' : 'bg-warn animate-pulse',
                        )}
                      />
                      <span className="font-mono text-ink-3">{r.account_id.slice(0, 6)}</span>
                      <span className="text-ink-3">·</span>
                      <span className="text-ink-2 uppercase tracking-[0.18em] text-[10px]">{r.state}</span>
                      {r.message && (
                        <>
                          <span className="text-ink-3">·</span>
                          <span className="ml-auto truncate text-ink-2 text-[10px]">{r.message}</span>
                        </>
                      )}
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

function NumInput({ label, value, onChange }: { label: string; value: number; onChange: (v: number) => void }) {
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
