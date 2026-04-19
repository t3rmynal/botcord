import { execFileSync } from 'node:child_process'
import { cp, mkdir, rm, stat } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '..')

async function exists(p) {
  try {
    await stat(p)
    return true
  } catch {
    return false
  }
}

async function main() {
  const exe = join(root, 'src-tauri/target/release/botcord.exe')
  if (!(await exists(exe))) {
    throw new Error(`not built: ${exe}. run: pnpm tauri build`)
  }

  const resChromium = join(root, 'src-tauri/resources/chromium')
  const resExtensions = join(root, 'src-tauri/resources/extensions')
  const resSidecar = join(root, 'src-tauri/sidecars/voice')

  for (const p of [resChromium, resExtensions, resSidecar]) {
    if (!(await exists(p))) throw new Error(`missing ${p}, run pnpm run setup and pnpm install`)
  }
  if (!(await exists(join(resSidecar, 'node_modules')))) {
    throw new Error('missing sidecar node_modules, run: pnpm --dir src-tauri/sidecars/voice install')
  }

  const out = join(root, 'dist-portable', 'botcord-portable')
  await rm(join(root, 'dist-portable'), { recursive: true, force: true })
  await mkdir(out, { recursive: true })
  await mkdir(join(out, 'resources'), { recursive: true })

  console.log('copy exe')
  await cp(exe, join(out, 'botcord.exe'))

  console.log('copy chromium')
  await cp(resChromium, join(out, 'resources', 'chromium'), { recursive: true })

  console.log('copy extensions')
  await cp(resExtensions, join(out, 'resources', 'extensions'), { recursive: true })

  console.log('copy sidecar')
  await cp(resSidecar, join(out, 'resources', 'sidecars', 'voice'), {
    recursive: true,
    dereference: true,
  })

  console.log('write readme')
  const readme = [
    'botcord portable',
    '',
    'requirements:',
    '- windows 11 (with WebView2 runtime, already shipped by ms)',
    '- node 20+ on PATH (for the voice sidecar)',
    '',
    'run botcord.exe. data stored in %APPDATA%/botcord.',
    '',
  ].join('\n')
  const { writeFile } = await import('node:fs/promises')
  await writeFile(join(out, 'README.txt'), readme)

  console.log('zip (bsdtar)')
  const zipPath = join(root, 'dist-portable', 'botcord-portable.zip')
  const tarExe =
    process.platform === 'win32'
      ? join(process.env.SystemRoot || 'C:\\Windows', 'System32', 'tar.exe')
      : 'tar'
  execFileSync(tarExe, ['-a', '-cf', zipPath, '-C', out, '.'], { stdio: 'inherit' })
  console.log('\nportable ready:', zipPath)
}

main().catch((e) => {
  console.error('fail:', e.message)
  process.exit(1)
})
