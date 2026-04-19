import { useEffect } from 'react'
import { Shell } from './components/Shell'
import { LockScreen } from './routes/LockScreen'
import { useVault } from './store/vault'

export function App() {
  const status = useVault((s) => s.status)
  const refresh = useVault((s) => s.refresh)

  useEffect(() => {
    refresh()
  }, [refresh])

  if (status === 'locked' || status === 'setup') {
    return <LockScreen />
  }
  if (status === 'unknown') {
    return (
      <div className="h-full grid place-items-center text-ink-3 text-[11px] uppercase tracking-[0.2em]">
        boot...
      </div>
    )
  }
  return <Shell />
}
