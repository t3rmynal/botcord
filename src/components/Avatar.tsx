import { useMemo } from 'react'

export function Avatar({
  id,
  hash,
  name,
  size = 28,
}: {
  id?: string | null
  hash?: string | null
  name?: string | null
  size?: number
}) {
  const url = useMemo(() => {
    if (!id || !hash) return null
    const ext = hash.startsWith('a_') ? 'gif' : 'png'
    return `https://cdn.discordapp.com/avatars/${id}/${hash}.${ext}?size=64`
  }, [id, hash])

  const initials = (name || '?').slice(0, 1).toLowerCase()

  return (
    <div
      className="grid place-items-center overflow-hidden text-[10px] font-mono shrink-0 border border-line-2 bg-bg-2 text-ink-2"
      style={{ width: size, height: size }}
    >
      {url ? <img src={url} alt="" className="w-full h-full object-cover" /> : initials}
    </div>
  )
}
