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
    if (hash && id) {
      const ext = hash.startsWith('a_') ? 'gif' : 'png'
      return `https://cdn.discordapp.com/avatars/${id}/${hash}.${ext}?size=128`
    }
    if (id) {
      const idx = defaultAvatarIndex(id)
      return `https://cdn.discordapp.com/embed/avatars/${idx}.png`
    }
    return null
  }, [id, hash])

  const initials = (name || '?').slice(0, 1).toLowerCase()

  return (
    <div
      className="grid place-items-center overflow-hidden text-[10px] font-mono shrink-0 border border-line-2 bg-bg-2 text-ink-2"
      style={{ width: size, height: size }}
    >
      {url ? (
        <img
          src={url}
          alt=""
          className="w-full h-full object-cover"
          onError={(e) => {
            ;(e.target as HTMLImageElement).style.display = 'none'
          }}
        />
      ) : (
        initials
      )}
    </div>
  )
}

function defaultAvatarIndex(userId: string): number {
  try {
    const n = BigInt(userId)
    return Number((n >> 22n) % 6n)
  } catch {
    let h = 0
    for (let i = 0; i < userId.length; i++) h = (h * 31 + userId.charCodeAt(i)) >>> 0
    return h % 6
  }
}
