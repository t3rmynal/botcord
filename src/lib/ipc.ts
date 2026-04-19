import { invoke } from '@tauri-apps/api/core'

export type AccountRow = {
  id: string
  discord_id: string | null
  label: string | null
  global_name: string | null
  avatar: string | null
  premium_type: number | null
  proxy_id: string | null
  valid: boolean | null
  last_check_at: number | null
}

export type CheckItem = {
  token_tail: string
  ok: boolean
  id: string | null
  username: string | null
  global_name: string | null
  avatar: string | null
  premium_type: number | null
  error: string | null
}

export type AddResult = {
  added: AccountRow[]
  skipped: string[]
}

export type ProxyRow = {
  id: string
  scheme: string
  host: string
  port: number
  has_auth: boolean
  shared_slots: number
  alive: boolean | null
  latency_ms: number | null
  last_check_at: number | null
  assigned_count: number
}

export type ProxyAddResult = {
  added: ProxyRow[]
  skipped: string[]
}

export const api = {
  accountsList: () => invoke<AccountRow[]>('accounts_list'),
  accountsCheck: (tokens: string[]) => invoke<CheckItem[]>('accounts_check', { tokens }),
  accountsAdd: (tokens: string[]) => invoke<AddResult>('accounts_add', { tokens }),
  accountsRemove: (id: string) => invoke<void>('accounts_remove', { id }),
  accountsRecheck: (ids: string[]) => invoke<AccountRow[]>('accounts_recheck', { ids }),

  proxiesList: () => invoke<ProxyRow[]>('proxies_list'),
  proxiesAdd: (raw: string) => invoke<ProxyAddResult>('proxies_add', { raw }),
  proxiesRemove: (id: string) => invoke<void>('proxies_remove', { id }),
  proxiesSetSlots: (id: string, slots: number) =>
    invoke<void>('proxies_set_slots', { id, slots }),
  proxiesTest: (id: string) => invoke<ProxyRow>('proxies_test', { id }),
  proxiesAssignAuto: () => invoke<number>('proxies_assign_auto'),
  proxiesAssign: (accountId: string, proxyId: string | null) =>
    invoke<void>('proxies_assign', { accountId, proxyId }),

  guildsList: () => invoke<GuildRow[]>('guilds_list'),
  voiceChannelsList: () => invoke<VoiceChannelRow[]>('voice_channels_list'),
  voiceChannelsAddManual: (channelId: string, guildId?: string, name?: string) =>
    invoke<void>('voice_channels_add_manual', {
      channelId,
      guildId: guildId ?? null,
      name: name ?? null,
    }),
  voiceChannelsRemove: (channelId: string) =>
    invoke<void>('voice_channels_remove', { channelId }),
  voiceChannelsSetFavorite: (channelId: string, favorite: boolean) =>
    invoke<void>('voice_channels_set_favorite', { channelId, favorite }),
  guildsImportFromAccount: (accountId: string) =>
    invoke<number>('guilds_import_from_account', { accountId }),

  voiceJoin: (accountId: string, channelId: string, mode: VoiceMode, guildId?: string) =>
    invoke<JoinResult>('voice_join', {
      args: { account_id: accountId, channel_id: channelId, guild_id: guildId ?? null, mode },
    }),
  voiceBulkJoin: (
    accountIds: string[],
    channelId: string,
    mode: VoiceMode,
    guildId?: string,
  ) =>
    invoke<JoinResult[]>('voice_bulk_join', {
      args: {
        account_ids: accountIds,
        channel_id: channelId,
        guild_id: guildId ?? null,
        mode,
      },
    }),
  voiceLeave: (accountId: string) => invoke<void>('voice_leave', { accountId }),
  voiceLeaveAll: (accountIds: string[]) =>
    invoke<void>('voice_leave_all', { accountIds }),

  browserOpen: (accountId: string) =>
    invoke<BrowserOpenResult>('browser_open', { accountId }),
  browserWipe: (accountId: string) =>
    invoke<void>('browser_wipe', { accountId }),

  broadcastDms: (args: {
    account_ids: string[]
    text: string
    image_path: string | null
    skip_groups: boolean
    min_delay_ms: number
    max_delay_ms: number
  }) => invoke<BroadcastResult>('broadcast_dms', { args }),
  broadcastGuilds: (args: {
    account_ids: string[]
    text: string
    image_path: string | null
    per_guild_limit: number
    skip_announcements: boolean
    min_delay_ms: number
    max_delay_ms: number
  }) => invoke<BroadcastResult>('broadcast_guilds', { args }),
  bulkSetProfile: (args: {
    account_ids: string[]
    global_name: string | null
    nickname_per_guild: string | null
    avatar_path: string | null
    min_delay_ms: number
    max_delay_ms: number
  }) => invoke<ProfileResult>('bulk_set_profile', { args }),
}

export type ProfileResult = {
  accounts_done: number
  guild_nicks_set: number
  failed: number
}

export type ProfileStep = {
  account_id: string
  action: string
  target: string | null
  ok: boolean
  error: string | null
}

export type BroadcastResult = {
  delivered: number
  failed: number
  skipped: number
}

export type BroadcastProgress = {
  account_id: string
  channel_id: string
  recipient: string | null
  ok: boolean
  error: string | null
}

export type BrowserOpenResult = {
  profile_dir: string
  chromium_path: string
  privacy_badger: boolean
}

export type VoiceMode = 'normal' | 'silent'

export type JoinResult = {
  account_id: string
  ok: boolean
  error: string | null
}

export type GuildRow = {
  guild_id: string
  name: string | null
  icon: string | null
}

export type VoiceChannelRow = {
  channel_id: string
  guild_id: string
  name: string | null
  favorite: boolean
}
