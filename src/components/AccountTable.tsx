import { useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { clsx } from 'clsx'
import { api } from '../lib/ipc'
import { useAccounts } from '../store/accounts'
import { Avatar } from './Avatar'
import { StatusPill } from './StatusPill'
import { NitroBadge } from './NitroBadge'

export function AccountTable({
  onExportAll,
  onRegister,
}: {
  onExportAll?: () => void
  onRegister?: () => void
}) {
  const list = useAccounts((s) => s.list)
  const selected = useAccounts((s) => s.selected)
  const toggle = useAccounts((s) => s.toggle)
  const toggleAll = useAccounts((s) => s.toggleAll)
  const remove = useAccounts((s) => s.remove)
  const recheck = useAccounts((s) => s.recheck)
  const [openingId, setOpeningId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const openInBrowser = async (id: string) => {
    setOpeningId(id)
    setError(null)
    try {
      await api.browserOpen(id)
    } catch (e) {
      setError(String(e))
      setTimeout(() => setError(null), 4000)
    } finally {
      setOpeningId(null)
    }
  }

  const allOn = list.length > 0 && list.every((a) => selected.has(a.id))

  if (list.length === 0) {
    return (
      <div className="mt-6 h-[160px] border border-dashed border-line-2 grid place-items-center text-ink-3 text-[11px] uppercase tracking-[0.2em]">
        // no accounts yet
      </div>
    )
  }

  return (
    <div className="mt-4 border border-line bg-bg/60 backdrop-blur-sm">
      <AnimatePresence>
        {error && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="px-4 py-2 bg-bad/5 text-bad text-[11px] border-b border-bad/30 overflow-hidden font-mono"
          >
            {'! '} {error}
          </motion.div>
        )}
      </AnimatePresence>
      <div className="h-9 px-3 grid grid-cols-[28px_1fr_160px_120px_100px_112px] items-center text-[10px] uppercase tracking-[0.22em] text-ink-3 border-b border-line">
        <Checkbox checked={allOn} onChange={(v) => toggleAll(v)} />
        <div>account</div>
        <div>proxy</div>
        <div>status</div>
        <div>last</div>
        <div className="flex items-center justify-end gap-1">
          {onRegister && (
            <button
              onClick={onRegister}
              className="h-6 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-2 hover:text-ink border border-line-2 hover:border-ink transition"
            >
              + register
            </button>
          )}
          {onExportAll && (
            <button
              onClick={onExportAll}
              className="h-6 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-3 hover:text-ink border border-transparent hover:border-line-3 transition"
            >
              export all
            </button>
          )}
        </div>
      </div>
      <AnimatePresence initial={false}>
        {list.map((a) => {
          const checked = selected.has(a.id)
          return (
            <motion.div
              key={a.id}
              layout
              initial={{ opacity: 0, y: 4 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, x: -12 }}
              transition={{ type: 'spring', stiffness: 320, damping: 30 }}
              className={clsx(
                'h-14 px-3 grid grid-cols-[28px_1fr_160px_120px_100px_112px] items-center border-b border-line/50 last:border-b-0',
                checked && 'bg-bg-3/40',
              )}
            >
              <Checkbox checked={checked} onChange={() => toggle(a.id)} />
              <div className="flex items-center gap-3 min-w-0">
                <Avatar id={a.discord_id} hash={a.avatar} name={a.label ?? a.global_name} />
                <div className="flex flex-col min-w-0">
                  <div className="text-[12px] truncate flex items-center gap-1.5">
                    {a.global_name || a.label || '—'}
                    <NitroBadge type={a.premium_type ?? null} />
                  </div>
                  <div className="text-[10px] text-ink-3 truncate font-mono">
                    {a.label ? `@${a.label}` : a.discord_id || a.id.slice(0, 8)}
                  </div>
                </div>
              </div>
              <div className="text-[11px] text-ink-3 truncate font-mono">
                {a.proxy_id ? (
                  <span>{a.proxy_id.slice(0, 8)}</span>
                ) : (
                  <span className="text-ink-4">—</span>
                )}
              </div>
              <div>
                <StatusPill state={a.valid === null ? 'unknown' : a.valid ? 'ok' : 'bad'} />
              </div>
              <div className="text-[11px] text-ink-3 font-mono">
                {a.last_check_at ? relTime(a.last_check_at) : '—'}
              </div>
              <div className="flex items-center gap-1 justify-end">
                <TextBtn
                  title="open in browser-tab"
                  loading={openingId === a.id}
                  onClick={() => openInBrowser(a.id)}
                >
                  browse
                </TextBtn>
                <TextBtn title="re-check" onClick={() => recheck([a.id])}>
                  check
                </TextBtn>
                <TextBtn title="remove" onClick={() => remove(a.id)} danger>
                  del
                </TextBtn>
              </div>
            </motion.div>
          )
        })}
      </AnimatePresence>
    </div>
  )
}

function Checkbox({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      onClick={() => onChange(!checked)}
      className={clsx(
        'w-4 h-4 border transition grid place-items-center',
        checked ? 'bg-ink border-ink text-bg' : 'border-line-2 hover:border-ink-3',
      )}
    >
      {checked && (
        <motion.svg
          initial={{ scale: 0.4, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          viewBox="0 0 16 16"
          width="10"
          height="10"
        >
          <path
            d="M3 8.5 6.5 12 13 4.5"
            stroke="currentColor"
            strokeWidth="2"
            fill="none"
            strokeLinecap="square"
            strokeLinejoin="miter"
          />
        </motion.svg>
      )}
    </button>
  )
}

function TextBtn({
  children,
  title,
  onClick,
  loading,
  danger,
}: {
  children: React.ReactNode
  title: string
  onClick?: () => void
  loading?: boolean
  danger?: boolean
}) {
  return (
    <button
      title={title}
      onClick={onClick}
      disabled={loading}
      className={clsx(
        'h-7 px-2 text-[10px] uppercase tracking-[0.2em] border border-transparent hover:border-line-3 transition disabled:opacity-40',
        danger ? 'text-bad/80 hover:text-bad' : 'text-ink-3 hover:text-ink',
      )}
    >
      {loading ? '...' : children}
    </button>
  )
}

function relTime(unix: number) {
  const d = Math.floor(Date.now() / 1000) - unix
  if (d < 60) return `${d}s`
  if (d < 3600) return `${Math.floor(d / 60)}m`
  if (d < 86400) return `${Math.floor(d / 3600)}h`
  return `${Math.floor(d / 86400)}d`
}
