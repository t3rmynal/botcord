import { useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { clsx } from 'clsx'
import { api, type InboxRow } from '../lib/ipc'
import { useAccounts } from '../store/accounts'

function randomId(len = 6): string {
  const alphabet = 'abcdefghijkmnopqrstuvwxyz23456789'
  let out = ''
  for (let i = 0; i < len; i++) out += alphabet[Math.floor(Math.random() * alphabet.length)]
  return out
}

function fmtStamp(d: Date): string {
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

export function RegisterModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [username, setUsername] = useState('')
  const [displayName, setDisplayName] = useState('1337')
  const [dob, setDob] = useState('')
  const [useProxy, setUseProxy] = useState(true)
  const [useDomain, setUseDomain] = useState(false)
  const [inboxes, setInboxes] = useState<InboxRow[]>([])
  const [selectedInbox, setSelectedInbox] = useState<string>('')
  const [token, setToken] = useState('')
  const [saving, setSaving] = useState(false)
  const [openingBrowser, setOpeningBrowser] = useState(false)
  const [result, setResult] = useState<{ ok: boolean; msg: string } | null>(null)
  const refresh = useAccounts((s) => s.refresh)

  useEffect(() => {
    if (!open) return
    setResult(null)
    setToken('')
    api.inboxesList().then((list) => {
      const withDomain = list.filter((i) => i.domain)
      setInboxes(withDomain)
      if (withDomain.length > 0 && !selectedInbox) {
        setSelectedInbox(withDomain[0].id)
        setUseDomain(true)
      }
    })
    regenerate()
  }, [open])

  const currentDomain = (): string | null => {
    if (!useDomain) return null
    const inbox = inboxes.find((i) => i.id === selectedInbox)
    return inbox?.domain || null
  }

  const regenerate = async () => {
    const p = await api.accountsRegisterPrepare(null)
    setPassword(p.password)
    setUsername(p.username)
    setDob(p.date_of_birth)
    const dom = currentDomain()
    if (dom) {
      setEmail(`reg-${randomId(6)}@${dom}`)
    }
  }

  useEffect(() => {
    if (useDomain) {
      const dom = currentDomain()
      if (dom) setEmail(`reg-${randomId(6)}@${dom}`)
    }
  }, [useDomain, selectedInbox])

  const openBrowser = async () => {
    if (!email.trim()) {
      setResult({ ok: false, msg: 'email empty, fill or enable custom domain' })
      return
    }
    setOpeningBrowser(true)
    try {
      await api.accountsRegisterViaBrowser({
        email: email.trim(),
        password,
        username,
        date_of_birth: dob,
        display_name: displayName.trim() || '1337',
        use_proxy: useProxy,
      })
      setResult({ ok: true, msg: 'chromium opened. register, copy token from devtools, paste below.' })
    } catch (e) {
      setResult({ ok: false, msg: String(e) })
    } finally {
      setOpeningBrowser(false)
    }
  }

  const saveToken = async () => {
    if (!token.trim()) return
    setSaving(true)
    setResult(null)
    try {
      const tok = token.trim().replace(/^"|"$/g, '')
      const r = await api.accountsAdd([tok])
      if (r.added.length > 0) {
        setResult({ ok: true, msg: `saved as ${r.added[0].label}` })
        await refresh()
        setTimeout(onClose, 1500)
      } else {
        setResult({ ok: false, msg: r.skipped[0] || 'token invalid' })
      }
    } catch (e) {
      setResult({ ok: false, msg: String(e) })
    } finally {
      setSaving(false)
    }
  }

  const copyCredentials = async () => {
    const now = fmtStamp(new Date())
    const dom = currentDomain()
    const lines = [
      `# registered ${now}`,
      `email: ${email}`,
      `password: ${password}`,
      `username: ${username}`,
      `display: ${displayName}`,
      `dob: ${dob}`,
    ]
    if (dom) lines.push(`domain: ${dom}`)
    await navigator.clipboard.writeText(lines.join('\n'))
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
            className="w-[620px] max-w-full max-h-[90vh] overflow-y-auto border border-line-3 bg-bg"
          >
            <div className="h-10 px-4 flex items-center gap-3 border-b border-line sticky top-0 bg-bg z-10">
              <span className="text-[11px] uppercase tracking-[0.22em]">register</span>
              <button
                onClick={regenerate}
                className="h-7 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-3 hover:text-ink border border-transparent hover:border-line-3 transition"
              >
                regenerate
              </button>
              <button
                onClick={copyCredentials}
                className="h-7 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-3 hover:text-ink border border-transparent hover:border-line-3 transition"
              >
                copy creds
              </button>
              <button
                className="ml-auto h-7 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-3 hover:text-ink transition"
                onClick={onClose}
              >
                esc
              </button>
            </div>
            <div className="p-4 flex flex-col gap-3">
              <div className="flex items-center gap-3">
                <label className="flex items-center gap-2 cursor-pointer h-8">
                  <input
                    type="checkbox"
                    checked={useDomain}
                    onChange={(e) => setUseDomain(e.target.checked)}
                    className="accent-white"
                    disabled={inboxes.length === 0}
                  />
                  <span className="text-[10px] uppercase tracking-[0.22em] text-ink-2">
                    custom domain
                  </span>
                </label>
                {useDomain && inboxes.length > 0 && (
                  <select
                    value={selectedInbox}
                    onChange={(e) => setSelectedInbox(e.target.value)}
                    className="h-8 px-2 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[11px] font-mono flex-1"
                  >
                    {inboxes.map((i) => (
                      <option key={i.id} value={i.id}>
                        {i.name} · @{i.domain}
                      </option>
                    ))}
                  </select>
                )}
                {inboxes.length === 0 && (
                  <span className="text-[10px] text-ink-4 uppercase tracking-[0.22em]">
                    add inbox with domain first
                  </span>
                )}
              </div>

              <Field label="email">
                <input
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="reg-xxx@your-domain.com"
                  autoComplete="off"
                  className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                />
              </Field>
              <div className="grid grid-cols-2 gap-3">
                <Field label="username">
                  <input
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                  />
                </Field>
                <Field label="display name">
                  <input
                    value={displayName}
                    onChange={(e) => setDisplayName(e.target.value)}
                    className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                  />
                </Field>
              </div>
              <Field label="password">
                <input
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono"
                />
              </Field>

              <label className="flex items-center gap-2 cursor-pointer h-8">
                <input
                  type="checkbox"
                  checked={useProxy}
                  onChange={(e) => setUseProxy(e.target.checked)}
                  className="accent-white"
                />
                <span className="text-[10px] uppercase tracking-[0.22em] text-ink-2">
                  use proxy + auto-assign after save
                </span>
              </label>

              <button
                onClick={openBrowser}
                disabled={openingBrowser}
                className="h-9 px-5 self-start border border-line-3 hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.22em] disabled:opacity-30 transition whitespace-nowrap"
              >
                {openingBrowser ? 'opening...' : '→ open chromium & register'}
              </button>

              <div className="border border-line bg-bg-2/40 p-3 flex flex-col gap-1 text-[11px] text-ink-2 leading-relaxed">
                <div className="text-[10px] uppercase tracking-[0.22em] text-ink-3 mb-1">
                  get token after register
                </div>
                <div>1. finish register in chromium (pick dob, solve captcha, submit)</div>
                <div>2. on any discord page press <span className="text-ink font-mono">F12</span></div>
                <div>3. tab <span className="text-ink font-mono">Application</span> → Local Storage → discord.com → key <span className="text-ink font-mono">token</span></div>
                <div>4. copy the value, paste below</div>
              </div>

              <Field label="paste token">
                <textarea
                  value={token}
                  onChange={(e) => setToken(e.target.value)}
                  spellCheck={false}
                  placeholder='"MTAxxxx.Gxxx.xxxxxxxxxxxx" or without quotes'
                  className="min-h-[72px] max-h-[160px] p-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[12px] font-mono resize-y"
                />
              </Field>

              <button
                onClick={saveToken}
                disabled={saving || !token.trim()}
                className="h-9 self-end px-5 border border-line-3 hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.22em] disabled:opacity-30 transition whitespace-nowrap"
              >
                {saving ? 'saving...' : '→ save account'}
              </button>

              {result && (
                <div
                  className={clsx(
                    'text-[11px] border-l-2 pl-2 py-1 font-mono break-words',
                    result.ok ? 'text-ok border-ok' : 'text-bad border-bad',
                  )}
                >
                  {result.msg}
                </div>
              )}
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="label">{label}</span>
      {children}
    </label>
  )
}
