import { motion } from 'framer-motion'

export function ShimmerText({
  text,
  className,
  duration = 2.8,
}: {
  text: string
  className?: string
  duration?: number
}) {
  return (
    <motion.span
      className={className}
      style={{
        backgroundImage:
          'linear-gradient(90deg, #5e5e5e 0%, #ffffff 45%, #ffffff 55%, #5e5e5e 100%)',
        backgroundSize: '200% 100%',
        WebkitBackgroundClip: 'text',
        WebkitTextFillColor: 'transparent',
        backgroundClip: 'text',
        color: 'transparent',
      }}
      animate={{ backgroundPositionX: ['200%', '-200%'] }}
      transition={{ duration, repeat: Infinity, ease: 'linear' }}
    >
      {text}
    </motion.span>
  )
}
