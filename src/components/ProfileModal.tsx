import { useEffect, useRef, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { clsx } from 'clsx'
import { api, type ProfileResult, type ProfileStep } from '../lib/ipc'

export function ProfileModal({
  open,
  onClose,
  accountIds,
}: {
  open: boolean
  onClose: () => void
  accountIds: string[]
}) {
  const [globalName, setGlobalName] = useState('')
  const [nick, setNick] = useState('')
  const [avatarPath, setAvatarPath] = useState<string | null>(null)
  const [minDelay, setMinDelay] = useState(800)
  const [maxDelay, setMaxDelay] = useState(2200)
  const [running, setRunning] = useState(false)
  const [progress, setProgress] = useState<ProfileStep[]>([])
  const [result, setResult] = useState<ProfileResult | null>(null)
  const listRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    let un: UnlistenFn | null = null
    listen<ProfileStep>('profile:progress', (e) => {
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

  const pickAvatar = async () => {
    const path = await openDialog({
      multiple: false,
      filters: [{ name: 'image', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
    })
    if (typeof path === 'string') setAvatarPath(path)
  }

  const noInputs = !globalName.trim() && !nick.trim() && !avatarPath

  const fire = async () => {
    if (running || noInputs || accountIds.length === 0) return
    setRunning(true)
    setProgress([])
    setResult(null)
    try {
      const r = await api.bulkSetProfile({
        account_ids: accountIds,
        global_name: globalName.trim() || null,
        nickname_per_guild: nick.trim() || null,
        avatar_path: avatarPath,
        min_delay_ms: minDelay,
        max_delay_ms: maxDelay,
      })
      setResult(r)
    } catch (e) {
      setProgress((p) => [
        ...p,
        { account_id: '?', action: 'fatal', target: null, ok: false, error: String(e) },
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
            className="w-[560px] max-w-full border border-line-3 bg-bg overflow-hidden flex flex-col"
          >
            <div className="h-10 px-4 flex items-center gap-3 border-b border-line">
              <span className="text-[11px] uppercase tracking-[0.22em]">mass profile</span>
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
              <Field
                label="display name"
                hint="what others see in chat, not your @username"
              >
                <input
                  value={globalName}
                  onChange={(e) => setGlobalName(e.target.value)}
                  placeholder="optional"
                  className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                />
              </Field>

              <Field label="nickname (applied in every guild)">
                <input
                  value={nick}
                  onChange={(e) => setNick(e.target.value)}
                  placeholder="optional"
                  className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                />
              </Field>

              <div className="grid grid-cols-[1fr_auto] gap-2 items-end">
                <Field label="avatar image">
                  <div className="h-9 px-3 bg-bg-2 border border-line-2 flex items-center text-[11px] text-ink-3 font-mono truncate">
                    {avatarPath || '—'}
                  </div>
                </Field>
                <div className="flex gap-1">
                  <button
                    onClick={pickAvatar}
                    className="h-9 px-3 border border-line-3 hover:border-ink text-[11px] uppercase tracking-[0.22em] transition"
                  >
                    pick
                  </button>
                  {avatarPath && (
                    <button
                      onClick={() => setAvatarPath(null)}
                      className="h-9 px-3 border border-line hover:border-bad text-[11px] uppercase tracking-[0.22em] text-bad/80 hover:text-bad transition"
                    >
                      ×
                    </button>
                  )}
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <NumInput label="min delay ms" value={minDelay} onChange={setMinDelay} />
                <NumInput label="max delay ms" value={maxDelay} onChange={setMaxDelay} />
              </div>

              <div className="flex items-center justify-between pt-1">
                <div className="text-[10px] uppercase tracking-[0.22em] text-ink-3">
                  {running
                    ? `running... ${progress.length}`
                    : result
                    ? `done ${result.accounts_done} / nicks ${result.guild_nicks_set} / fail ${result.failed}`
                    : 'ready'}
                </div>
                <button
                  onClick={fire}
                  disabled={running || accountIds.length === 0 || noInputs}
                  className="h-9 px-5 border border-line-3 hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.22em] disabled:opacity-30 transition"
                >
                  {running ? 'applying...' : '→ apply'}
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
                      <span className="text-ink-2">{r.action}</span>
                      {r.target && (
                        <>
                          <span className="text-ink-3">·</span>
                          <span className="truncate text-ink-2">{r.target}</span>
                        </>
                      )}
                      <span
                        className={clsx(
                          'ml-auto text-[10px] uppercase tracking-[0.18em]',
                          r.ok ? 'text-ok' : 'text-bad',
                        )}
                      >
                        {r.ok ? 'ok' : r.error?.slice(0, 40) || 'fail'}
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

function Field({
  label,
  hint,
  children,
}: {
  label: string
  hint?: string
  children: React.ReactNode
}) {
  return (
    <label className="flex flex-col gap-1.5">
      <div className="flex items-baseline gap-2">
        <span className="label">{label}</span>
        {hint && <span className="text-[10px] text-ink-4 normal-case tracking-normal">{hint}</span>}
      </div>
      {children}
    </label>
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
