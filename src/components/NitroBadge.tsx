import { clsx } from 'clsx'

export function NitroBadge({ type }: { type: number | null }) {
  const has = (type ?? 0) > 0
  const label =
    type === 1 ? 'classic' : type === 3 ? 'basic' : has ? 'nitro' : 'no nitro'
  return (
    <span
      className={clsx(
        'inline-flex items-center gap-1 h-4 px-1.5 border text-[9px] uppercase tracking-[0.18em]',
        has ? 'border-ink-2 text-ink' : 'border-ink-4 text-ink-4',
      )}
    >
      <span className={clsx('w-1 h-1', has ? 'bg-ink' : 'bg-ink-4')} />
      {label}
    </span>
  )
}
