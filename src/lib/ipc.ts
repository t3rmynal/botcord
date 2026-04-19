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

export type SocialResult = {
  ok: number
  failed: number
  captcha_pending: number
}

export type SocialStep = {
  account_id: string
  state: string
  message: string | null
}

export type RegisterPrepared = {
  email: string
  password: string
  username: string
  date_of_birth: string
}

export type RegisterResult = {
  ok: boolean
  account: AccountRow | null
  error: string | null
  prepared: RegisterPrepared
}

export type BrowserRegisterResult = {
  ok: boolean
  account: AccountRow | null
  error: string | null
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
  accountsRegisterPrepare: (email?: string | null) =>
    invoke<RegisterPrepared>('accounts_register_prepare', { email: email ?? null }),
  accountsRegisterWithCaptcha: (args: {
    email: string
    password: string
    username: string
    date_of_birth: string
    invite: string | null
    use_proxy: boolean
  }) => invoke<RegisterResult>('accounts_register_with_captcha', { args }),
  accountsRegisterViaBrowser: (args: {
    email: string
    password: string
    username: string
    date_of_birth: string
    display_name: string | null
    use_proxy: boolean
  }) => invoke<BrowserRegisterResult>('accounts_register_via_browser', { args }),
  accountsExportTokens: (ids: string[] | null) =>
    invoke<string[]>('accounts_export_tokens', { ids }),
  saveTextFile: (path: string, content: string) =>
    invoke<void>('save_text_file', { path, content }),

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
  serversClearAll: () => invoke<void>('servers_clear_all'),

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

  bulkJoinInvite: (args: {
    account_ids: string[]
    invite: string
    min_delay_ms: number
    max_delay_ms: number
  }) => invoke<SocialResult>('bulk_join_invite', { args }),
  bulkFriendRequest: (args: {
    account_ids: string[]
    username: string
    discriminator: string | null
    min_delay_ms: number
    max_delay_ms: number
  }) => invoke<SocialResult>('bulk_friend_request', { args }),

  browserOpen: (accountId: string) =>
    invoke<BrowserOpenResult>('browser_open', { accountId }),
  browserWipe: (accountId: string) =>
    invoke<void>('browser_wipe', { accountId }),
  inboxOpen: () => invoke<string>('inbox_open'),
  inboxWipe: () => invoke<void>('inbox_wipe'),
  inboxesList: () => invoke<InboxRow[]>('inboxes_list'),
  inboxesAdd: (name: string, url: string, domain: string | null) =>
    invoke<InboxRow>('inboxes_add', { name, url, domain }),
  inboxesRemove: (id: string) => invoke<void>('inboxes_remove', { id }),
  inboxesOpen: (id: string) => invoke<void>('inboxes_open', { id }),

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
    reset_avatar: boolean
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

export type InboxRow = {
  id: string
  name: string
  url: string
  domain: string | null
  created_at: number
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
