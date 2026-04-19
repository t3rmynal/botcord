import { useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { clsx } from 'clsx'
import { useProxies } from '../store/proxies'
import { useAccounts } from '../store/accounts'
import { ProxySlotSlider } from '../components/ProxySlotSlider'

export function Proxies() {
  const list = useProxies((s) => s.list)
  const adding = useProxies((s) => s.adding)
  const lastSkipped = useProxies((s) => s.lastSkipped)
  const refresh = useProxies((s) => s.refresh)
  const add = useProxies((s) => s.add)
  const remove = useProxies((s) => s.remove)
  const setSlots = useProxies((s) => s.setSlots)
  const test = useProxies((s) => s.test)
  const assignAuto = useProxies((s) => s.assignAuto)
  const refreshAccounts = useAccounts((s) => s.refresh)

  const [raw, setRaw] = useState('')
  const [assigning, setAssigning] = useState(false)
  const [assignedMsg, setAssignedMsg] = useState<string | null>(null)

  useEffect(() => {
    refresh().catch(() => {})
  }, [refresh])

  const onAdd = async () => {
    if (!raw.trim()) return
    await add(raw)
    setRaw('')
  }

  const onAssign = async () => {
    setAssigning(true)
    try {
      const n = await assignAuto()
      setAssignedMsg(`assigned ${n}`)
      await refresh()
      await refreshAccounts()
      setTimeout(() => setAssignedMsg(null), 2500)
    } finally {
      setAssigning(false)
    }
  }

  return (
    <div className="h-full overflow-auto">
      <div className="p-6 flex flex-col gap-4">
        <div className="border border-line bg-bg/60 backdrop-blur-sm">
          <div className="h-9 px-4 border-b border-line flex items-center text-[11px] uppercase tracking-[0.2em] text-ink-2">
            add proxies
          </div>
          <div className="p-4 flex flex-col gap-3">
            <textarea
              value={raw}
              onChange={(e) => setRaw(e.target.value)}
              spellCheck={false}
              placeholder={'> ip:port\n> ip:port:user:pass\n> http://user:pass@ip:port\n> socks5://ip:port'}
              className="min-h-[100px] max-h-[200px] p-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono resize-y"
            />
            <div className="flex items-center gap-2">
              <button
                onClick={onAdd}
                disabled={adding || !raw.trim()}
                className="h-8 px-4 border border-line-3 hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.2em] disabled:opacity-30 transition"
              >
                {adding ? 'adding...' : 'add'}
              </button>
              <button
                onClick={onAssign}
                disabled={assigning || list.length === 0}
                className="h-8 px-4 border border-line hover:border-ink text-[11px] uppercase tracking-[0.2em] disabled:opacity-30 transition"
              >
                {assigning ? 'assigning...' : 'auto-assign'}
              </button>
              {assignedMsg && (
                <span className="text-[10px] uppercase tracking-[0.2em] text-ok">
                  ok · {assignedMsg}
                </span>
              )}
            </div>
            <AnimatePresence>
              {lastSkipped.length > 0 && (
                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  className="flex items-start gap-2 p-2 border border-warn/30 text-[11px] text-warn bg-warn/5"
                >
                  <span>!</span>
                  <div className="flex flex-col gap-0.5">
                    <div className="uppercase tracking-[0.18em] text-[10px]">
                      skipped {lastSkipped.length}
                    </div>
                    <div className="text-ink-3 font-mono">{lastSkipped.slice(0, 5).join(' · ')}</div>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </div>

        {list.length === 0 ? (
          <div className="h-[160px] border border-dashed border-line-2 grid place-items-center text-ink-3 text-[11px] uppercase tracking-[0.2em]">
            // no proxies
          </div>
        ) : (
          <div className="flex flex-col gap-1">
            <AnimatePresence initial={false}>
              {list.map((p) => (
                <motion.div
                  key={p.id}
                  layout
                  initial={{ opacity: 0, y: 4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, x: -12 }}
                  transition={{ type: 'spring', stiffness: 320, damping: 30 }}
                  className="border border-line bg-bg/60 backdrop-blur-sm px-4 py-3 flex items-center gap-4"
                >
                  <div className="flex flex-col min-w-0 flex-1">
                    <div className="text-[12px] font-mono truncate">
                      {p.scheme}://{p.host}:{p.port}
                      {p.has_auth && <span className="text-ink-3"> · auth</span>}
                    </div>
                    <div className="text-[10px] text-ink-3 uppercase tracking-[0.2em] mt-0.5">
                      <span
                        className={clsx(
                          'mr-2',
                          p.alive === null
                            ? 'text-ink-3'
                            : p.alive
                            ? 'text-ok'
                            : 'text-bad',
                        )}
                      >
                        {p.alive === null ? 'untested' : p.alive ? 'alive' : 'dead'}
                      </span>
                      {p.latency_ms != null && <span className="text-ink-3">{p.latency_ms}ms · </span>}
                      <span className="text-ink-3">
                        {p.assigned_count}/{p.shared_slots}
                      </span>
                    </div>
                  </div>
                  <ProxySlotSlider
                    value={p.shared_slots}
                    used={p.assigned_count}
                    onChange={(v) => setSlots(p.id, v)}
                  />
                  <div className="flex items-center gap-1">
                    <TextBtn onClick={() => test(p.id)}>test</TextBtn>
                    <TextBtn onClick={() => remove(p.id)} danger>
                      del
                    </TextBtn>
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

function TextBtn({
  children,
  onClick,
  danger,
}: {
  children: React.ReactNode
  onClick?: () => void
  danger?: boolean
}) {
  return (
    <button
      onClick={onClick}
      className={clsx(
        'h-7 px-2 text-[10px] uppercase tracking-[0.2em] border border-transparent hover:border-line-3 transition',
        danger ? 'text-bad/80 hover:text-bad' : 'text-ink-3 hover:text-ink',
      )}
    >
      {children}
    </button>
  )
}
