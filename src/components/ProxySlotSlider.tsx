import { motion } from 'framer-motion'
import { clsx } from 'clsx'

export function ProxySlotSlider({
  value,
  used,
  onChange,
}: {
  value: number
  used: number
  onChange: (v: number) => void
}) {
  return (
    <div className="flex items-center gap-1">
      {Array.from({ length: 5 }).map((_, i) => {
        const n = i + 1
        const active = n <= value
        const filled = n <= used
        return (
          <button
            key={n}
            onClick={() => onChange(n)}
            className={clsx(
              'w-4 h-4 relative transition border',
              active ? 'border-ink-3' : 'border-line',
              'hover:border-ink',
            )}
            title={`${value} slots, ${used} used`}
          >
            {filled && (
              <motion.span
                layoutId={`slot-fill-${n}`}
                className="absolute inset-[2px] bg-ink"
                transition={{ type: 'spring', stiffness: 300, damping: 30 }}
              />
            )}
          </button>
        )
      })}
    </div>
  )
}
