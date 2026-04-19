import { AnimatePresence, motion } from 'framer-motion'
import { useAccounts } from '../store/accounts'
import { api } from '../lib/ipc'
import { clsx } from 'clsx'

export function BulkBar({
  onJoin,
  onBroadcast,
  onProfile,
  onExport,
  onInvite,
  onFriend,
}: {
  onJoin?: () => void
  onBroadcast?: () => void
  onProfile?: () => void
  onExport?: () => void
  onInvite?: () => void
  onFriend?: () => void
}) {
  const selected = useAccounts((s) => s.selected)
  const clear = useAccounts((s) => s.clearSelection)
  const recheck = useAccounts((s) => s.recheck)

  const n = selected.size
  const selectedIds = [...selected]

  const onRemoveAll = async () => {
    const { remove } = useAccounts.getState()
    for (const id of selectedIds) await remove(id)
  }

  const onLeaveAll = async () => {
    if (selectedIds.length === 0) return
    await api.voiceLeaveAll(selectedIds)
  }

  return (
    <AnimatePresence>
      {n > 0 && (
        <motion.div
          initial={{ y: 80, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: 80, opacity: 0 }}
          transition={{ type: 'spring', stiffness: 320, damping: 30 }}
          className="fixed bottom-4 left-1/2 -translate-x-1/2 h-11 pl-3 pr-2 border border-line-3 bg-bg-2/90 backdrop-blur flex items-center gap-1 shadow-[0_0_40px_rgba(0,0,0,0.6)] z-40"
        >
          <div className="px-2 text-[10px] uppercase tracking-[0.22em] text-ink-2">
            <span className="text-ink font-medium">{String(n).padStart(2, '0')}</span>
            <span className="text-ink-3 ml-2">selected</span>
          </div>
          <Divider />
          <Action label="join vc" onClick={onJoin} />
          <Action label="leave" onClick={onLeaveAll} />
          <Divider />
          <Action label="invite" onClick={onInvite} />
          <Action label="friend" onClick={onFriend} />
          <Action label="broadcast" onClick={onBroadcast} />
          <Action label="profile" onClick={onProfile} />
          <Action label="export" onClick={onExport} />
          <Action label="re-check" onClick={() => recheck(selectedIds)} />
          <Action label="del" onClick={onRemoveAll} danger />
          <Divider />
          <button
            onClick={() => clear()}
            className="h-7 px-2 text-[10px] uppercase tracking-[0.22em] text-ink-3 hover:text-ink transition"
            title="clear"
          >
            esc
          </button>
        </motion.div>
      )}
    </AnimatePresence>
  )
}

function Divider() {
  return <div className="w-px h-5 bg-line mx-1" />
}

function Action({
  label,
  onClick,
  danger,
}: {
  label: string
  onClick?: () => void
  danger?: boolean
}) {
  return (
    <button
      onClick={onClick}
      className={clsx(
        'h-7 px-3 text-[10px] uppercase tracking-[0.22em] border border-transparent hover:border-line-3 transition',
        danger ? 'text-bad/80 hover:text-bad' : 'text-ink-2 hover:text-ink',
      )}
    >
      {label}
    </button>
  )
}
