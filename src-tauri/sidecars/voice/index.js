'use strict'

const fs = require('fs')
const os = require('os')
const path = require('path')
const readline = require('readline')
const WebSocket = require('ws')
const { HttpsProxyAgent } = require('https-proxy-agent')
const { SocksProxyAgent } = require('socks-proxy-agent')

const LOG_PATH = path.join(os.tmpdir(), 'botcord-voice.log')
try {
  fs.writeFileSync(LOG_PATH, `--- sidecar start ${new Date().toISOString()} ---\n`)
} catch {}

function log(...args) {
  try {
    const line = args
      .map((a) => (typeof a === 'string' ? a : JSON.stringify(a)))
      .join(' ')
    fs.appendFileSync(LOG_PATH, `[${new Date().toISOString()}] ${line}\n`)
  } catch {}
}

const rl = readline.createInterface({ input: process.stdin })

const sessions = new Map()

function send(msg) {
  process.stdout.write(JSON.stringify(msg) + '\n')
}
function reply(id, result) {
  send({ id, result })
}
function fail(id, error) {
  const m = String(error && error.message ? error.message : error)
  log('rpc-fail', m)
  send({ id, error: m })
}
function emit(event, payload) {
  send({ event, payload })
}

function proxyUrl(p) {
  if (!p) return null
  const auth = p.user && p.pass ? `${encodeURIComponent(p.user)}:${encodeURIComponent(p.pass)}@` : ''
  return `${p.scheme}://${auth}${p.host}:${p.port}`
}

function buildAgent(p) {
  if (!p) return undefined
  const url = proxyUrl(p)
  if (p.scheme === 'socks5' || p.scheme === 'socks5h') return new SocksProxyAgent(url)
  return new HttpsProxyAgent(url)
}

function openGateway(token, identity, proxy, account_id) {
  return new Promise((resolve, reject) => {
    const url = 'wss://gateway.discord.gg/?v=10&encoding=json'
    const wsOpts = {}
    const agent = buildAgent(proxy)
    if (agent) wsOpts.agent = agent

    log('openGateway', { account_id, proxy: proxy ? `${proxy.scheme}://${proxy.host}:${proxy.port}` : null })
    let ws
    try {
      ws = new WebSocket(url, wsOpts)
    } catch (e) {
      log('ws ctor error', String(e && e.message || e))
      return reject(e)
    }

    let seq = null
    let hbTimer = null
    let resolved = false
    const timeout = setTimeout(() => {
      if (resolved) return
      try { ws.close() } catch {}
      reject(new Error('gateway handshake timeout'))
    }, 35000)

    ws.on('open', () => {
      log('ws open', account_id)
      emit('status', { account_id, state: 'connecting_gateway' })
    })

    ws.on('close', (code, reason) => {
      log('ws close', account_id, code, reason ? reason.toString() : '')
      if (hbTimer) clearInterval(hbTimer)
      sessions.delete(account_id)
      emit('status', {
        account_id,
        state: 'disconnected',
        code,
        reason: reason ? reason.toString() : undefined,
      })
      if (!resolved) {
        clearTimeout(timeout)
        reject(new Error(`gateway closed ${code} ${reason ? reason.toString() : ''}`))
      }
    })

    ws.on('error', (e) => {
      const msg = String(e && e.message || e)
      log('ws error', account_id, msg)
      emit('status', { account_id, state: 'gateway_error', message: msg })
      if (!resolved) {
        resolved = true
        clearTimeout(timeout)
        reject(new Error(`gateway error: ${msg}`))
      }
    })

    ws.on('message', (raw) => {
      let msg
      try {
        msg = JSON.parse(raw.toString())
      } catch {
        return
      }
      if (typeof msg.s === 'number') seq = msg.s

      log('ws recv', account_id, 'op=', msg.op, 't=', msg.t || '')
      switch (msg.op) {
        case 10: {
          const hb = msg.d.heartbeat_interval
          hbTimer = setInterval(() => {
            try {
              ws.send(JSON.stringify({ op: 1, d: seq }))
            } catch {}
          }, hb)

          const props = identity
            ? {
                os: identity.os,
                browser: identity.browser,
                device: identity.device || '',
                system_locale: identity.system_locale,
                browser_user_agent: identity.browser_user_agent,
                browser_version: identity.browser_version,
                os_version: identity.os_version,
                referrer: '',
                referring_domain: '',
                referrer_current: '',
                referring_domain_current: '',
                release_channel: identity.release_channel,
                client_build_number: identity.client_build_number,
                client_event_source: null,
              }
            : {}

          ws.send(
            JSON.stringify({
              op: 2,
              d: {
                token,
                capabilities: 16381,
                properties: props,
                presence: { status: 'online', since: 0, activities: [], afk: false },
                compress: false,
                client_state: { guild_versions: {} },
              },
            }),
          )
          emit('status', { account_id, state: 'identifying' })
          break
        }
        case 0: {
          if (msg.t === 'READY') {
            const session_id = msg.d.session_id
            sessions.set(account_id, {
              ws,
              session_id,
              hbTimer,
              get seq() { return seq },
              token,
            })
            emit('status', { account_id, state: 'logged_in' })
            if (!resolved) {
              resolved = true
              clearTimeout(timeout)
              resolve({ ws, session_id })
            }
          }
          break
        }
        case 9: {
          if (!resolved) {
            resolved = true
            clearTimeout(timeout)
            try { ws.close() } catch {}
            reject(new Error('invalid session on identify, token likely dead or needs verification'))
          }
          break
        }
      }
    })
  })
}

async function handleJoin(params) {
  const { account_id, token, identity, proxy, guild_id, channel_id, mode } = params
  if (!token) throw new Error('no token')
  if (!guild_id) throw new Error('no guild_id')
  if (!channel_id) throw new Error('no channel_id')

  let sess = sessions.get(account_id)
  if (!sess || sess.ws.readyState !== WebSocket.OPEN) {
    await openGateway(token, identity, proxy, account_id)
    sess = sessions.get(account_id)
  }
  if (!sess) throw new Error('gateway session missing after connect')

  emit('status', { account_id, state: 'joining_voice' })
  sess.ws.send(
    JSON.stringify({
      op: 4,
      d: {
        guild_id,
        channel_id,
        self_mute: mode === 'silent',
        self_deaf: mode === 'silent',
      },
    }),
  )
  emit('status', { account_id, state: 'ready', guild_id, channel_id, mode })
  return { ok: true }
}

async function handleLeave(params) {
  const { account_id } = params
  const sess = sessions.get(account_id)
  if (!sess) return { ok: true }
  try {
    sess.ws.send(
      JSON.stringify({
        op: 4,
        d: { guild_id: null, channel_id: null, self_mute: false, self_deaf: false },
      }),
    )
  } catch {}
  emit('status', { account_id, state: 'left' })
  return { ok: true }
}

async function handleLogout(params) {
  const { account_id, token } = params
  const target = account_id || null
  for (const [aid, s] of sessions.entries()) {
    if (target ? aid === target : s.token === token) {
      try { s.ws.close() } catch {}
      if (s.hbTimer) clearInterval(s.hbTimer)
      sessions.delete(aid)
    }
  }
  return { ok: true }
}

function handleStatus() {
  const out = []
  for (const [aid, s] of sessions.entries()) {
    out.push({
      account_id: aid,
      state: s.ws.readyState === WebSocket.OPEN ? 'connected' : 'closed',
    })
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
      case 'join': result = await handleJoin(params); break
      case 'leave': result = await handleLeave(params); break
      case 'logout': result = await handleLogout(params); break
      case 'status': result = handleStatus(); break
      case 'ping': result = { pong: true }; break
      default: throw new Error('unknown method: ' + method)
    }
    reply(id, result)
  } catch (e) {
    fail(id, e)
  }
})

process.on('uncaughtException', (e) => {
  log('uncaught', String(e && e.stack || e))
  emit('status', { account_id: null, state: 'uncaught', message: String(e && e.message || e) })
})
process.on('unhandledRejection', (e) => {
  log('rejection', String(e && e.stack || e))
  emit('status', { account_id: null, state: 'rejection', message: String(e && e.message || e) })
})

log('sidecar ready, node', process.version)
emit('ready', { version: '0.3.0', log: LOG_PATH })
