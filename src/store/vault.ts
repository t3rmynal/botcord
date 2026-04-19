import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'

type Status = 'unknown' | 'setup' | 'locked' | 'unlocked'

interface VaultState {
  status: Status
  error: string | null
  refresh: () => Promise<void>
  setup: (password: string) => Promise<void>
  unlock: (password: string) => Promise<void>
  lock: () => Promise<void>
}

export const useVault = create<VaultState>((set) => ({
  status: 'unknown',
  error: null,
  refresh: async () => {
    const s = await invoke<Status>('vault_status')
    set({ status: s, error: null })
  },
  setup: async (password) => {
    try {
      await invoke('vault_setup', { password })
      set({ status: 'unlocked', error: null })
    } catch (e) {
      set({ error: String(e) })
    }
  },
  unlock: async (password) => {
    try {
      await invoke('vault_unlock', { password })
      set({ status: 'unlocked', error: null })
    } catch (e) {
      set({ error: String(e) })
    }
  },
  lock: async () => {
    await invoke('vault_lock')
    set({ status: 'locked', error: null })
  },
}))
