import { create } from 'zustand'
import { api, type GuildRow, type VoiceChannelRow } from '../lib/ipc'

interface ServersState {
  guilds: GuildRow[]
  channels: VoiceChannelRow[]
  importing: boolean
  refresh: () => Promise<void>
  addManual: (channelId: string, guildId?: string, name?: string) => Promise<void>
  remove: (channelId: string) => Promise<void>
  toggleFavorite: (channelId: string) => Promise<void>
  importFromAccount: (accountId: string) => Promise<number>
  clearAll: () => Promise<void>
}

export const useServers = create<ServersState>((set, get) => ({
  guilds: [],
  channels: [],
  importing: false,

  refresh: async () => {
    const [guilds, channels] = await Promise.all([api.guildsList(), api.voiceChannelsList()])
    set({ guilds, channels })
  },
  addManual: async (channelId, guildId, name) => {
    await api.voiceChannelsAddManual(channelId, guildId, name)
    await get().refresh()
  },
  remove: async (channelId) => {
    await api.voiceChannelsRemove(channelId)
    set({ channels: get().channels.filter((c) => c.channel_id !== channelId) })
  },
  toggleFavorite: async (channelId) => {
    const ch = get().channels.find((c) => c.channel_id === channelId)
    if (!ch) return
    const next = !ch.favorite
    await api.voiceChannelsSetFavorite(channelId, next)
    set({
      channels: get().channels.map((c) =>
        c.channel_id === channelId ? { ...c, favorite: next } : c,
      ),
    })
  },
  importFromAccount: async (accountId) => {
    set({ importing: true })
    try {
      const n = await api.guildsImportFromAccount(accountId)
      await get().refresh()
      return n
    } finally {
      set({ importing: false })
    }
  },
  clearAll: async () => {
    await api.serversClearAll()
    set({ guilds: [], channels: [] })
  },
}))
