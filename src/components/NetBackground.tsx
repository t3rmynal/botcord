import { useEffect, useRef } from 'react'

type Pt = { x: number; y: number; vx: number; vy: number }

export function NetBackground({ density = 0.00009 }: { density?: number }) {
  const ref = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = ref.current
    if (!canvas) return
    const ctx = canvas.getContext('2d', { alpha: true })
    if (!ctx) return

    const dpr = Math.min(window.devicePixelRatio || 1, 2)
    let w = 0
    let h = 0
    let pts: Pt[] = []
    const mouse = { x: -9999, y: -9999, active: false }

    const resize = () => {
      w = canvas.clientWidth
      h = canvas.clientHeight
      canvas.width = Math.floor(w * dpr)
      canvas.height = Math.floor(h * dpr)
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
      const target = Math.max(40, Math.floor(w * h * density))
      if (pts.length < target) {
        while (pts.length < target) pts.push(spawn(w, h))
      } else if (pts.length > target) {
        pts.length = target
      }
    }

    const spawn = (w: number, h: number): Pt => ({
      x: Math.random() * w,
      y: Math.random() * h,
      vx: (Math.random() - 0.5) * 0.15,
      vy: (Math.random() - 0.5) * 0.15,
    })

    resize()
    window.addEventListener('resize', resize)

    const onMove = (e: MouseEvent) => {
      const r = canvas.getBoundingClientRect()
      mouse.x = e.clientX - r.left
      mouse.y = e.clientY - r.top
      mouse.active = true
    }
    const onLeave = () => {
      mouse.active = false
      mouse.x = -9999
      mouse.y = -9999
    }
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseleave', onLeave)

    const linkDist = 130
    const repelRadius = 120

    let raf = 0
    const tick = () => {
      ctx.clearRect(0, 0, w, h)

      for (const p of pts) {
        if (mouse.active) {
          const dx = p.x - mouse.x
          const dy = p.y - mouse.y
          const d2 = dx * dx + dy * dy
          if (d2 < repelRadius * repelRadius && d2 > 0.01) {
            const d = Math.sqrt(d2)
            const f = (1 - d / repelRadius) * 0.4
            p.vx += (dx / d) * f
            p.vy += (dy / d) * f
          }
        }
        p.vx *= 0.985
        p.vy *= 0.985
        p.x += p.vx
        p.y += p.vy
        if (p.x < -10) p.x = w + 10
        if (p.x > w + 10) p.x = -10
        if (p.y < -10) p.y = h + 10
        if (p.y > h + 10) p.y = -10
      }

      ctx.lineWidth = 1
      for (let i = 0; i < pts.length; i++) {
        for (let j = i + 1; j < pts.length; j++) {
          const a = pts[i]
          const b = pts[j]
          const dx = a.x - b.x
          const dy = a.y - b.y
          const d2 = dx * dx + dy * dy
          if (d2 < linkDist * linkDist) {
            const d = Math.sqrt(d2)
            const o = 1 - d / linkDist
            ctx.strokeStyle = `rgba(255,255,255,${(o * 0.12).toFixed(3)})`
            ctx.beginPath()
            ctx.moveTo(a.x, a.y)
            ctx.lineTo(b.x, b.y)
            ctx.stroke()
          }
        }
      }

      if (mouse.active) {
        for (const p of pts) {
          const dx = p.x - mouse.x
          const dy = p.y - mouse.y
          const d2 = dx * dx + dy * dy
          if (d2 < linkDist * linkDist) {
            const d = Math.sqrt(d2)
            const o = 1 - d / linkDist
            ctx.strokeStyle = `rgba(255,255,255,${(o * 0.35).toFixed(3)})`
            ctx.beginPath()
            ctx.moveTo(p.x, p.y)
            ctx.lineTo(mouse.x, mouse.y)
            ctx.stroke()
          }
        }
      }

      for (const p of pts) {
        ctx.fillStyle = 'rgba(255,255,255,0.65)'
        ctx.beginPath()
        ctx.arc(p.x, p.y, 1.1, 0, Math.PI * 2)
        ctx.fill()
      }

      raf = requestAnimationFrame(tick)
    }
    tick()

    return () => {
      cancelAnimationFrame(raf)
      window.removeEventListener('resize', resize)
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseleave', onLeave)
    }
  }, [density])

  return (
    <canvas
      ref={ref}
      className="fixed inset-0 w-full h-full pointer-events-none opacity-70"
      style={{ zIndex: 0 }}
    />
  )
}
