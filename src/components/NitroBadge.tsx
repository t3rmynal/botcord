export function NitroBadge({ type }: { type: number | null }) {
  if (!type) return null
  const label =
    type === 1 ? 'classic' : type === 2 ? 'nitro' : type === 3 ? 'basic' : 'nitro'
  return (
    <span className="inline-flex items-center gap-1 h-4 px-1.5 border border-ink-3 text-[9px] uppercase tracking-[0.18em] text-ink-2">
      <span className="w-1 h-1 bg-ink-2" />
      {label}
    </span>
  )
}
