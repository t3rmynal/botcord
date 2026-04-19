import { useEffect, useState } from 'react'
import { TokenCheckCard } from '../components/TokenCheckCard'
import { AccountTable } from '../components/AccountTable'
import { BulkBar } from '../components/BulkBar'
import { JoinVcModal } from '../components/JoinVcModal'
import { BroadcastModal } from '../components/BroadcastModal'
import { ProfileModal } from '../components/ProfileModal'
import { useAccounts } from '../store/accounts'

export function Accounts() {
  const refresh = useAccounts((s) => s.refresh)
  const selected = useAccounts((s) => s.selected)
  const [joinOpen, setJoinOpen] = useState(false)
  const [broadcastOpen, setBroadcastOpen] = useState(false)
  const [profileOpen, setProfileOpen] = useState(false)

  useEffect(() => {
    refresh().catch(() => {})
  }, [refresh])

  return (
    <div className="relative h-full overflow-auto">
      <div className="p-6 flex flex-col gap-4 pb-24">
        <TokenCheckCard />
        <AccountTable />
      </div>
      <BulkBar
        onJoin={() => setJoinOpen(true)}
        onBroadcast={() => setBroadcastOpen(true)}
        onProfile={() => setProfileOpen(true)}
      />
      <JoinVcModal
        open={joinOpen}
        onClose={() => setJoinOpen(false)}
        accountIds={[...selected]}
      />
      <BroadcastModal
        open={broadcastOpen}
        onClose={() => setBroadcastOpen(false)}
        accountIds={[...selected]}
      />
      <ProfileModal
        open={profileOpen}
        onClose={() => setProfileOpen(false)}
        accountIds={[...selected]}
      />
    </div>
  )
}
