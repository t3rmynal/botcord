import { getCurrentWindow } from '@tauri-apps/api/window'
import { ShimmerText } from './ShimmerText'

const win = getCurrentWindow()

export function Titlebar({ right }: { right?: React.ReactNode }) {
  return (
    <div
      data-tauri-drag-region
      className="h-8 pl-3 pr-0 flex items-center select-none border-b border-line bg-bg"
    >
      <div className="pointer-events-none flex items-baseline gap-2">
        <ShimmerText text="botcord" className="text-[12px] font-semibold tracking-tight" />
        <span className="text-[9px] text-ink-4">v0.1.0</span>
      </div>
      <div data-tauri-drag-region className="flex-1 h-full" />
      <div className="flex items-center">
        {right && <div className="pr-1 flex items-center">{right}</div>}
        <WinBtn label="—" onClick={() => win.minimize()} title="minimize" />
        <WinBtn label="▢" onClick={() => win.toggleMaximize()} title="maximize" />
        <WinBtn label="×" onClick={() => win.close()} title="close" danger />
      </div>
    </div>
  )
}

function WinBtn({
  label,
  onClick,
  title,
  danger,
}: {
  label: string
  onClick: () => void
  title: string
  danger?: boolean
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      className={
        'w-10 h-8 grid place-items-center text-[11px] text-ink-3 transition ' +
        (danger ? 'hover:bg-bad hover:text-white' : 'hover:bg-bg-3 hover:text-ink')
      }
    >
      {label}
    </button>
  )
}
