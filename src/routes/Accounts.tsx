import { useEffect, useState } from 'react'
import { TokenCheckCard } from '../components/TokenCheckCard'
import { AccountTable } from '../components/AccountTable'
import { BulkBar } from '../components/BulkBar'
import { JoinVcModal } from '../components/JoinVcModal'
import { BroadcastModal } from '../components/BroadcastModal'
import { ProfileModal } from '../components/ProfileModal'
import { ExportTokensModal } from '../components/ExportTokensModal'
import { RegisterModal } from '../components/RegisterModal'
import { InviteModal } from '../components/InviteModal'
import { FriendModal } from '../components/FriendModal'
import { useAccounts } from '../store/accounts'

export function Accounts() {
  const refresh = useAccounts((s) => s.refresh)
  const selected = useAccounts((s) => s.selected)
  const [joinOpen, setJoinOpen] = useState(false)
  const [broadcastOpen, setBroadcastOpen] = useState(false)
  const [profileOpen, setProfileOpen] = useState(false)
  const [registerOpen, setRegisterOpen] = useState(false)
  const [inviteOpen, setInviteOpen] = useState(false)
  const [friendOpen, setFriendOpen] = useState(false)
  const [exportIds, setExportIds] = useState<string[] | null | undefined>(undefined)

  useEffect(() => {
    refresh().catch(() => {})
  }, [refresh])

  return (
    <div className="relative h-full overflow-auto">
      <div className="p-6 flex flex-col gap-4 pb-24">
        <TokenCheckCard />
        <AccountTable
          onExportAll={() => setExportIds(null)}
          onRegister={() => setRegisterOpen(true)}
        />
      </div>
      <BulkBar
        onJoin={() => setJoinOpen(true)}
        onBroadcast={() => setBroadcastOpen(true)}
        onProfile={() => setProfileOpen(true)}
        onExport={() => setExportIds([...selected])}
        onInvite={() => setInviteOpen(true)}
        onFriend={() => setFriendOpen(true)}
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
      <ExportTokensModal
        open={exportIds !== undefined}
        onClose={() => setExportIds(undefined)}
        ids={exportIds === undefined ? null : exportIds}
      />
      <RegisterModal open={registerOpen} onClose={() => setRegisterOpen(false)} />
      <InviteModal open={inviteOpen} onClose={() => setInviteOpen(false)} accountIds={[...selected]} />
      <FriendModal open={friendOpen} onClose={() => setFriendOpen(false)} accountIds={[...selected]} />
    </div>
  )
}
