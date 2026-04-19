import { useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { clsx } from 'clsx'
import { Accounts } from '../routes/Accounts'
import { Proxies } from '../routes/Proxies'
import { Servers } from '../routes/Servers'
import { Inbox } from '../routes/Inbox'
import { useVault } from '../store/vault'
import { NetBackground } from './NetBackground'
import { Titlebar } from './Titlebar'

type Tab = 'accounts' | 'proxies' | 'servers' | 'inbox'

const tabs: { id: Tab; label: string; short: string }[] = [
  { id: 'accounts', label: 'accounts', short: 'acc' },
  { id: 'proxies', label: 'proxies', short: 'prx' },
  { id: 'servers', label: 'servers', short: 'srv' },
  { id: 'inbox', label: 'inbox', short: 'inb' },
]

export function Shell() {
  const [tab, setTab] = useState<Tab>('accounts')
  const lock = useVault((s) => s.lock)

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'l') {
        e.preventDefault()
        lock()
      }
    }
    window.addEventListener('keydown', h)
    return () => window.removeEventListener('keydown', h)
  }, [lock])

  return (
    <div className="relative h-full bg-bg crt flex flex-col">
      <NetBackground />
      <div className="relative z-10 flex flex-col h-full">
        <Titlebar
          right={
            <button
              onClick={() => lock()}
              className="h-7 mx-1 px-2 border border-line hover:border-line-3 text-[10px] uppercase tracking-widest text-ink-2 hover:text-ink transition"
              title="ctrl+l"
            >
              lock
            </button>
          }
        />

        <div className="flex-1 grid grid-cols-[160px_1fr] min-h-0">
          <nav className="border-r border-line p-2 flex flex-col gap-0.5 bg-bg/60 backdrop-blur-sm">
            {tabs.map((t) => {
              const active = tab === t.id
              return (
                <button
                  key={t.id}
                  onClick={() => setTab(t.id)}
                  className={clsx(
                    'relative h-8 px-3 flex items-center justify-between text-[11px] uppercase tracking-widest transition',
                    active ? 'text-ink' : 'text-ink-3 hover:text-ink-2',
                  )}
                >
                  {active && (
                    <motion.span
                      layoutId="nav-bar"
                      transition={{ type: 'spring', stiffness: 400, damping: 34 }}
                      className="absolute left-0 top-0 bottom-0 w-[3px] bg-ink"
                    />
                  )}
                  <span className="relative">{t.label}</span>
                  <span className="relative text-ink-4 text-[9px]">{t.short}</span>
                </button>
              )
            })}
          </nav>

          <main className="overflow-hidden bg-bg/40 backdrop-blur-sm min-h-0">
            <AnimatePresence mode="wait">
              <motion.div
                key={tab}
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -6 }}
                transition={{ duration: 0.12 }}
                className="h-full"
              >
                {tab === 'accounts' && <Accounts />}
                {tab === 'proxies' && <Proxies />}
                {tab === 'servers' && <Servers />}
                {tab === 'inbox' && <Inbox />}
              </motion.div>
            </AnimatePresence>
          </main>
        </div>
      </div>
    </div>
  )
}
