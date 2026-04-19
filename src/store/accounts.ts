import { create } from 'zustand'
import { api, type AccountRow, type CheckItem } from '../lib/ipc'

interface AccountsState {
  list: AccountRow[]
  selected: Set<string>
  checking: boolean
  lastTokens: string[] | null
  lastCheck: CheckItem[] | null
  adding: boolean

  refresh: () => Promise<void>
  toggle: (id: string) => void
  toggleAll: (on: boolean) => void
  clearSelection: () => void
  check: (tokens: string) => Promise<void>
  clearLastCheck: () => void
  addValid: () => Promise<void>
  remove: (id: string) => Promise<void>
  recheck: (ids: string[]) => Promise<void>
}

function parseTokens(raw: string): string[] {
  return raw
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean)
}

export const useAccounts = create<AccountsState>((set, get) => ({
  list: [],
  selected: new Set(),
  checking: false,
  lastTokens: null,
  lastCheck: null,
  adding: false,

  refresh: async () => {
    const list = await api.accountsList()
    set({ list })
  },
  toggle: (id) => {
    const s = new Set(get().selected)
    if (s.has(id)) s.delete(id)
    else s.add(id)
    set({ selected: s })
  },
  toggleAll: (on) => {
    if (on) set({ selected: new Set(get().list.map((a) => a.id)) })
    else set({ selected: new Set() })
  },
  clearSelection: () => set({ selected: new Set() }),

  check: async (tokensRaw) => {
    const tokens = parseTokens(tokensRaw)
    if (tokens.length === 0) return
    set({ checking: true, lastTokens: tokens, lastCheck: null })
    try {
      const items = await api.accountsCheck(tokens)
      set({ lastCheck: items })
    } finally {
      set({ checking: false })
    }
  },
  clearLastCheck: () => set({ lastCheck: null, lastTokens: null }),

  addValid: async () => {
    const last = get().lastCheck
    const tokens = get().lastTokens
    if (!last || !tokens) return
    const validTokens = tokens.filter((_, i) => last[i]?.ok)
    if (validTokens.length === 0) return
    set({ adding: true })
    try {
      const res = await api.accountsAdd(validTokens)
      const list = [...get().list, ...res.added]
      set({ list, lastCheck: null, lastTokens: null })
    } finally {
      set({ adding: false })
    }
  },

  remove: async (id) => {
    await api.accountsRemove(id)
    const list = get().list.filter((a) => a.id !== id)
    const sel = new Set(get().selected)
    sel.delete(id)
    set({ list, selected: sel })
  },
  recheck: async (ids) => {
    const list = await api.accountsRecheck(ids)
    set({ list })
  },
}))
