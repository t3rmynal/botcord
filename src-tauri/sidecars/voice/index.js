'use strict'

const readline = require('readline')
const { Client } = require('discord.js-selfbot-v13')
const voice = require('@discordjs/voice')
const { ProxyAgent } = require('undici')
const { SocksProxyAgent } = require('socks-proxy-agent')

const rl = readline.createInterface({ input: process.stdin })
const clients = new Map()
const connections = new Map()

function send(msg) {
  process.stdout.write(JSON.stringify(msg) + '\n')
}
function reply(id, result) {
  send({ id, result })
}
function fail(id, error) {
  send({ id, error: String(error && error.message ? error.message : error) })
}
function emit(event, payload) {
  send({ event, payload })
}

function proxyUrl(p) {
  if (!p) return null
  const auth = p.user && p.pass ? `${encodeURIComponent(p.user)}:${encodeURIComponent(p.pass)}@` : ''
  return `${p.scheme}://${auth}${p.host}:${p.port}`
}

function buildHttpAgent(p) {
  if (!p) return null
  const url = proxyUrl(p)
  if (p.scheme === 'socks5' || p.scheme === 'socks5h') return new SocksProxyAgent(url)
  return new ProxyAgent(url)
}

function buildClient(token, identity, proxy) {
  const opts = {
    checkUpdate: false,
    ws: {
      properties: identity
        ? {
            os: identity.os,
            browser: identity.browser,
            device: identity.device || '',
            system_locale: identity.system_locale,
            browser_user_agent: identity.browser_user_agent,
            browser_version: identity.browser_version,
            os_version: identity.os_version,
            release_channel: identity.release_channel,
            client_build_number: identity.client_build_number,
            client_event_source: null,
          }
        : undefined,
    },
    http: {},
  }
  const agent = buildHttpAgent(proxy)
  if (agent) {
    opts.http.agent = agent
  }
  return new Client(opts)
}

async function login(token, identity, proxy) {
  const existing = clients.get(token)
  if (existing && existing.readyTimestamp) return existing
  const client = buildClient(token, identity, proxy)
  await client.login(token)
  clients.set(token, client)
  return client
}

async function handleJoin(params) {
  const { account_id, token, identity, proxy, guild_id, channel_id, mode } = params
  const client = await login(token, identity, proxy)
  const guild = client.guilds.cache.get(guild_id) || (await client.guilds.fetch(guild_id).catch(() => null))
  if (!guild) throw new Error('not in guild')
  const selfMute = mode === 'silent'
  const selfDeaf = mode === 'silent'

  const connection = voice.joinVoiceChannel({
    channelId: channel_id,
    guildId: guild_id,
    adapterCreator: guild.voiceAdapterCreator,
    selfMute,
    selfDeaf,
  })

  connection.on('stateChange', (_old, next) => {
    emit('status', { account_id, state: next.status })
  })

  connection.on('error', (e) => {
    emit('status', { account_id, state: 'error', message: String(e && e.message) })
  })

  try {
    await voice.entersState(connection, voice.VoiceConnectionStatus.Ready, 15_000)
  } catch (e) {
    try {
      connection.destroy()
    } catch {}
    throw new Error('voice connect timeout')
  }

  connections.set(account_id, { connection, token })
  emit('status', { account_id, state: 'ready', channel_id, guild_id, mode })
  return { ok: true }
}

async function handleLeave(params) {
  const { account_id } = params
  const entry = connections.get(account_id)
  if (!entry) return { ok: true }
  try {
    entry.connection.destroy()
  } catch {}
  connections.delete(account_id)
  emit('status', { account_id, state: 'left' })
  return { ok: true }
}

async function handleLogout(params) {
  const { token } = params
  const c = clients.get(token)
  if (c) {
    try {
      c.destroy()
    } catch {}
    clients.delete(token)
  }
  for (const [aid, entry] of connections.entries()) {
    if (entry.token === token) {
      try {
        entry.connection.destroy()
      } catch {}
      connections.delete(aid)
    }
  }
  return { ok: true }
}

function handleStatus() {
  const out = []
  for (const [aid, e] of connections.entries()) {
    out.push({ account_id: aid, state: e.connection.state.status })
  }
  return { statuses: out }
}

rl.on('line', async (line) => {
  let msg
  try {
    msg = JSON.parse(line)
  } catch {
    return
  }
  const { id, method, params } = msg
  try {
    let result
    switch (method) {
      case 'join':
        result = await handleJoin(params)
        break
      case 'leave':
        result = await handleLeave(params)
        break
      case 'logout':
        result = await handleLogout(params)
        break
      case 'status':
        result = handleStatus()
        break
      case 'ping':
        result = { pong: true }
        break
      default:
        throw new Error('unknown method: ' + method)
    }
    reply(id, result)
  } catch (e) {
    fail(id, e)
  }
})

emit('ready', { version: '0.1.0' })
