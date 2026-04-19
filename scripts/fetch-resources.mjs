import { createWriteStream } from 'node:fs'
import { mkdir, mkdtemp, readdir, rename, rm, stat } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { pipeline } from 'node:stream/promises'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '..')

const GH_HEADERS = { 'User-Agent': 'botcord-setup' }
if (process.env.GH_TOKEN) GH_HEADERS.Authorization = `Bearer ${process.env.GH_TOKEN}`

async function exists(p) {
  try {
    await stat(p)
    return true
  } catch {
    return false
  }
}

async function newTmp(label) {
  return await mkdtemp(join(tmpdir(), `botcord-${label}-`))
}

async function download(url, outPath) {
  console.log(`    download ${url}`)
  const r = await fetch(url, { redirect: 'follow' })
  if (!r.ok) throw new Error(`${r.status} ${r.statusText} for ${url}`)
  await pipeline(r.body, createWriteStream(outPath))
}

async function ghLatest(repo, match) {
  const r = await fetch(`https://api.github.com/repos/${repo}/releases/latest`, {
    headers: GH_HEADERS,
  })
  if (!r.ok) {
    const body = await r.text().catch(() => '')
    throw new Error(`github ${repo}: ${r.status} ${body.slice(0, 120)}`)
  }
  const data = await r.json()
  const asset = data.assets.find((a) => match.test(a.name))
  if (!asset) {
    const names = data.assets.map((a) => a.name).join(', ')
    throw new Error(`no asset matches ${match} in ${repo} (${data.tag_name}). got: ${names}`)
  }
  return { url: asset.browser_download_url, name: asset.name, tag: data.tag_name }
}

function unzip(zipPath, outDir) {
  if (process.platform === 'win32') {
    execFileSync(
      'powershell',
      [
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-Command',
        `Expand-Archive -LiteralPath '${zipPath.replace(/'/g, "''")}' -DestinationPath '${outDir.replace(/'/g, "''")}' -Force`,
      ],
      { stdio: 'inherit' },
    )
  } else {
    execFileSync('unzip', ['-q', '-o', zipPath, '-d', outDir], { stdio: 'inherit' })
  }
}

async function adoptDir(srcRoot, target) {
  await rm(target, { recursive: true, force: true })
  await mkdir(dirname(target), { recursive: true })
  const items = await readdir(srcRoot)
  const only = items.length === 1 ? join(srcRoot, items[0]) : null
  if (only) {
    const st = await stat(only)
    if (st.isDirectory()) {
      await rename(only, target)
      return
    }
  }
  await rename(srcRoot, target)
}

async function ensureChromium() {
  if (process.platform !== 'win32') {
    console.log('chromium: auto-fetch supported on windows only. drop a portable build manually into src-tauri/resources/chromium/<platform>/')
    return
  }
  const target = join(root, 'src-tauri/resources/chromium/win-x64')
  if (await exists(join(target, 'chrome.exe'))) {
    console.log('chromium: already present')
    return
  }
  console.log('chromium: fetching ungoogled-chromium windows x64...')
  const asset = await ghLatest(
    'ungoogled-software/ungoogled-chromium-windows',
    /windows_x64\.zip$/i,
  )
  console.log(`    ${asset.tag} / ${asset.name}`)
  const tmp = await newTmp('uc')
  const zipPath = join(tmp, asset.name)
  await download(asset.url, zipPath)
  const work = join(tmp, 'unpack')
  await mkdir(work, { recursive: true })
  unzip(zipPath, work)
  await adoptDir(work, target)
  await rm(tmp, { recursive: true, force: true })
  console.log(`chromium: installed -> ${target}`)
}

async function ensurePrivacyBadger() {
  const target = join(root, 'src-tauri/resources/extensions/privacy-badger')
  if (await exists(join(target, 'manifest.json'))) {
    console.log('privacy-badger: already present')
    return
  }

  console.log('privacy-badger: shallow clone eff/privacybadger...')
  const tmp = await newTmp('pb')
  const clonePath = join(tmp, 'pb')
  execFileSync(
    'git',
    ['clone', '--depth=1', 'https://github.com/EFForg/privacybadger.git', clonePath],
    { stdio: 'inherit' },
  )
  const src = join(clonePath, 'src')
  if (!(await exists(join(src, 'manifest.json')))) {
    throw new Error('upstream src/manifest.json missing')
  }
  await rm(target, { recursive: true, force: true })
  await mkdir(dirname(target), { recursive: true })
  await rename(src, target)
  await rm(tmp, { recursive: true, force: true })
  console.log(`privacy-badger: installed -> ${target}`)
}

try {
  await ensureChromium()
  await ensurePrivacyBadger()
  console.log('\nresources ok. run: pnpm tauri dev')
} catch (e) {
  console.error('\nsetup failed:', e.message)
  process.exit(1)
}
