import { clsx } from 'clsx'

type State = 'ok' | 'bad' | 'unknown'

export function StatusPill({ state, label }: { state: State; label?: string }) {
  return (
    <span
      className={clsx(
        'inline-flex items-center gap-1.5 px-1.5 h-5 text-[10px] uppercase tracking-[0.18em] border',
        state === 'ok' && 'border-ok/50 text-ok',
        state === 'bad' && 'border-bad/50 text-bad',
        state === 'unknown' && 'border-ink-4 text-ink-3',
      )}
    >
      <span
        className={clsx(
          'w-1.5 h-1.5',
          state === 'ok' && 'bg-ok',
          state === 'bad' && 'bg-bad',
          state === 'unknown' && 'bg-ink-3',
        )}
      />
      {label ?? (state === 'ok' ? 'valid' : state === 'bad' ? 'invalid' : 'unknown')}
    </span>
  )
}
