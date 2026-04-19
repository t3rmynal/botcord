import { useState } from 'react'
import { motion } from 'framer-motion'
import { useVault } from '../store/vault'
import { NetBackground } from '../components/NetBackground'
import { Titlebar } from '../components/Titlebar'

export function LockScreen() {
  const status = useVault((s) => s.status)
  const error = useVault((s) => s.error)
  const setup = useVault((s) => s.setup)
  const unlock = useVault((s) => s.unlock)

  const [pw, setPw] = useState('')
  const [pw2, setPw2] = useState('')
  const [busy, setBusy] = useState(false)

  const isSetup = status === 'setup'

  async function submit(e: React.FormEvent) {
    e.preventDefault()
    if (busy) return
    if (isSetup && pw !== pw2) return
    if (pw.length < 4) return
    setBusy(true)
    try {
      if (isSetup) await setup(pw)
      else await unlock(pw)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="relative h-full bg-bg crt flex flex-col">
      <NetBackground />
      <div className="relative z-10 flex flex-col h-full">
        <Titlebar />
        <div className="flex-1 grid place-items-center">
        <motion.form
          onSubmit={submit}
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ type: 'spring', stiffness: 240, damping: 26 }}
          className="w-[400px] border border-line bg-bg/80 backdrop-blur-sm p-6 flex flex-col gap-5"
        >
          <div className="label">{isSetup ? 'init vault' : 'unlock'}</div>

          <div className="flex flex-col gap-2">
            <LabeledInput
              label="password"
              value={pw}
              onChange={setPw}
              autoFocus
              type="password"
            />
            {isSetup && (
              <LabeledInput
                label="repeat"
                value={pw2}
                onChange={setPw2}
                type="password"
              />
            )}
          </div>

          {error && (
            <div className="text-[11px] text-bad border-l-2 border-bad pl-3 py-1">
              {error}
            </div>
          )}

          <button
            type="submit"
            disabled={busy}
            className="h-9 border border-line hover:border-ink bg-bg/60 hover:bg-bg-3 text-[11px] uppercase tracking-[0.2em] disabled:opacity-50 transition"
          >
            {busy ? '...' : isSetup ? 'create vault' : 'unlock'}
          </button>

          {isSetup && (
            <div className="label">
              this password encrypts tokens + proxies locally. we cant recover it.
            </div>
          )}
        </motion.form>
        </div>
      </div>
    </div>
  )
}

function LabeledInput({
  label,
  value,
  onChange,
  type = 'text',
  autoFocus,
}: {
  label: string
  value: string
  onChange: (v: string) => void
  type?: string
  autoFocus?: boolean
}) {
  return (
    <label className="flex flex-col gap-1">
      <span className="label">{label}</span>
      <input
        autoFocus={autoFocus}
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="h-9 px-3 bg-bg-2 border border-line-2 outline-none focus:border-ink text-[13px] font-mono"
      />
    </label>
  )
}
