import { create } from 'zustand'
import { api, type ProxyRow } from '../lib/ipc'

interface ProxiesState {
  list: ProxyRow[]
  adding: boolean
  lastSkipped: string[]
  refresh: () => Promise<void>
  add: (raw: string) => Promise<void>
  remove: (id: string) => Promise<void>
  setSlots: (id: string, slots: number) => Promise<void>
  test: (id: string) => Promise<void>
  assignAuto: () => Promise<number>
}

export const useProxies = create<ProxiesState>((set, get) => ({
  list: [],
  adding: false,
  lastSkipped: [],

  refresh: async () => {
    const list = await api.proxiesList()
    set({ list })
  },
  add: async (raw) => {
    set({ adding: true, lastSkipped: [] })
    try {
      const res = await api.proxiesAdd(raw)
      set({
        list: [...get().list, ...res.added],
        lastSkipped: res.skipped,
      })
    } finally {
      set({ adding: false })
    }
  },
  remove: async (id) => {
    await api.proxiesRemove(id)
    set({ list: get().list.filter((p) => p.id !== id) })
  },
  setSlots: async (id, slots) => {
    await api.proxiesSetSlots(id, slots)
    set({
      list: get().list.map((p) => (p.id === id ? { ...p, shared_slots: slots } : p)),
    })
  },
  test: async (id) => {
    const row = await api.proxiesTest(id)
    set({ list: get().list.map((p) => (p.id === id ? row : p)) })
  },
  assignAuto: async () => {
    const n = await api.proxiesAssignAuto()
    return n
  },
}))
