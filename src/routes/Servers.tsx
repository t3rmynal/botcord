import { useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { clsx } from 'clsx'
import { useServers } from '../store/servers'
import { useAccounts } from '../store/accounts'

export function Servers() {
  const guilds = useServers((s) => s.guilds)
  const channels = useServers((s) => s.channels)
  const importing = useServers((s) => s.importing)
  const refresh = useServers((s) => s.refresh)
  const addManual = useServers((s) => s.addManual)
  const remove = useServers((s) => s.remove)
  const toggleFav = useServers((s) => s.toggleFavorite)
  const importFromAccount = useServers((s) => s.importFromAccount)

  const accounts = useAccounts((s) => s.list)
  const refreshAccounts = useAccounts((s) => s.refresh)

  const [cid, setCid] = useState('')
  const [gid, setGid] = useState('')
  const [chName, setChName] = useState('')
  const [importAcc, setImportAcc] = useState<string>('')
  const [importMsg, setImportMsg] = useState<string | null>(null)

  useEffect(() => {
    refresh().catch(() => {})
    refreshAccounts().catch(() => {})
  }, [refresh, refreshAccounts])

  useEffect(() => {
    if (!importAcc && accounts[0]) setImportAcc(accounts[0].id)
  }, [accounts, importAcc])

  const onAdd = async () => {
    if (!cid.trim()) return
    await addManual(cid.trim(), gid.trim() || undefined, chName.trim() || undefined)
    setCid('')
    setGid('')
    setChName('')
  }

  const onImport = async () => {
    if (!importAcc) return
    const n = await importFromAccount(importAcc)
    setImportMsg(`imported ${n}`)
    setTimeout(() => setImportMsg(null), 2500)
  }

  const grouped: Record<string, typeof channels> = {}
  for (const ch of channels) {
    const k = ch.guild_id || '__loose__'
    ;(grouped[k] ||= []).push(ch)
  }

  return (
    <div className="h-full overflow-auto">
      <div className="p-6 flex flex-col gap-4">
        <div className="grid grid-cols-2 gap-3">
          <div className="border border-line bg-bg/60 backdrop-blur-sm">
            <div className="h-9 px-4 border-b border-line flex items-center text-[11px] uppercase tracking-[0.2em] text-ink-2">
              add channel by id
            </div>
            <div className="p-4 flex flex-col gap-2">
              <MonoInput value={cid} onChange={setCid} placeholder="channel id" />
              <MonoInput value={gid} onChange={setGid} placeholder="guild id (optional)" />
              <MonoInput value={chName} onChange={setChName} placeholder="label (optional)" mono={false} />
              <button
                onClick={onAdd}
                disabled={!cid.trim()}
                className="h-8 self-start px-4 border border-line-3 hover:border-ink text-[11px] uppercase tracking-[0.2em] disabled:opacity-30 transition"
              >
                add
              </button>
            </div>
          </div>

          <div className="border border-line bg-bg/60 backdrop-blur-sm">
            <div className="h-9 px-4 border-b border-line flex items-center text-[11px] uppercase tracking-[0.2em] text-ink-2">
              import from token
            </div>
            <div className="p-4 flex flex-col gap-2">
              <select
                value={importAcc}
                onChange={(e) => setImportAcc(e.target.value)}
                className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px]"
              >
                <option value="">pick account...</option>
                {accounts.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.global_name || a.label || a.discord_id?.slice(0, 10) || a.id.slice(0, 6)}
                  </option>
                ))}
              </select>
              <button
                onClick={onImport}
                disabled={importing || !importAcc}
                className="h-8 self-start px-4 border border-line-3 hover:border-ink text-[11px] uppercase tracking-[0.2em] disabled:opacity-30 transition"
              >
                {importing ? 'importing...' : 'fetch voice channels'}
              </button>
              {importMsg && (
                <span className="text-[10px] uppercase tracking-[0.2em] text-ok">ok · {importMsg}</span>
              )}
            </div>
          </div>
        </div>

        {channels.length === 0 ? (
          <div className="h-[160px] border border-dashed border-line-2 grid place-items-center text-ink-3 text-[11px] uppercase tracking-[0.2em]">
            // no channels
          </div>
        ) : (
          <div className="flex flex-col gap-3">
            {Object.entries(grouped).map(([gid, chs]) => {
              const g = guilds.find((x) => x.guild_id === gid)
              return (
                <div key={gid} className="border border-line bg-bg/60 backdrop-blur-sm">
                  <div className="h-9 px-3 flex items-center gap-3 border-b border-line">
                    <span className="text-[11px] uppercase tracking-[0.2em]">
                      {g?.name || (gid === '__loose__' ? 'manual' : gid.slice(0, 14))}
                    </span>
                    {gid !== '__loose__' && <CopyId label={gid} />}
                    <span className="ml-auto text-[10px] text-ink-3">{chs.length} ch</span>
                  </div>
                  <div>
                    <AnimatePresence initial={false}>
                      {chs.map((ch) => (
                        <motion.div
                          key={ch.channel_id}
                          layout
                          initial={{ opacity: 0, y: 4 }}
                          animate={{ opacity: 1, y: 0 }}
                          exit={{ opacity: 0, x: -12 }}
                          className="h-11 px-3 flex items-center gap-3 border-b border-line/50 last:border-b-0"
                        >
                          <span className="w-4 text-ink-3 text-[12px]">#</span>
                          <div className="flex flex-col min-w-0">
                            <div className="text-[12px] truncate">
                              {ch.name || ch.channel_id.slice(0, 12)}
                            </div>
                            <div className="text-[10px] text-ink-3 font-mono">{ch.channel_id}</div>
                          </div>
                          <div className="ml-auto flex items-center gap-1">
                            <button
                              onClick={() => toggleFav(ch.channel_id)}
                              className={clsx(
                                'h-7 px-2 text-[10px] uppercase tracking-[0.2em] border border-transparent hover:border-line-3 transition',
                                ch.favorite ? 'text-ink' : 'text-ink-3 hover:text-ink',
                              )}
                              title="favorite"
                            >
                              {ch.favorite ? '★' : '☆'}
                            </button>
                            <CopyId label={ch.channel_id} />
                            <button
                              onClick={() => remove(ch.channel_id)}
                              title="remove"
                              className="h-7 px-2 text-[10px] uppercase tracking-[0.2em] text-bad/80 hover:text-bad border border-transparent hover:border-line-3 transition"
                            >
                              del
                            </button>
                          </div>
                        </motion.div>
                      ))}
                    </AnimatePresence>
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
}

function MonoInput({
  value,
  onChange,
  placeholder,
  mono = true,
}: {
  value: string
  onChange: (v: string) => void
  placeholder: string
  mono?: boolean
}) {
  return (
    <input
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className={clsx(
        'h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px]',
        mono && 'font-mono',
      )}
    />
  )
}

function CopyId({ label }: { label: string }) {
  const [copied, setCopied] = useState(false)
  return (
    <button
      onClick={async () => {
        await navigator.clipboard.writeText(label)
        setCopied(true)
        setTimeout(() => setCopied(false), 1000)
      }}
      title={copied ? 'copied' : 'copy id'}
      className="h-7 px-2 text-[10px] uppercase tracking-[0.2em] text-ink-3 hover:text-ink border border-transparent hover:border-line-3 transition"
    >
      {copied ? '✓ copied' : 'copy id'}
    </button>
  )
}
