<script lang="ts">
  import { EFFECT, type Effect } from './extensions'

  let { snow }: { snow: Effect } = $props()

  const HEADER = 54
  const STATUS = 26
  const BIN = 4
  const PILE = 7

  type Flake = { x: number; y: number; z: number; r: number; vy: number; amp: number; ph: number; f: number; rot: number }

  let canvas: HTMLCanvasElement
  const o = $derived({
    density: snow.density ?? EFFECT.density[2],
    speed: snow.speed ?? EFFECT.speed[2],
    size: snow.size ?? EFFECT.size[2],
    wind: snow.wind ?? EFFECT.wind[2],
    header: snow.area === 'header',
    crystals: snow.shape === 'crystals',
    pile: !!snow.pile && snow.area !== 'header',
  })

  $effect(() => {
    void o.density
    void o.header
    const ctx = canvas.getContext('2d')!
    let w = 0
    let h = 0
    let flakes: Flake[] = []
    let pile = new Float32Array(0)
    let frame = 0
    let last = 0

    const spawn = (top: boolean): Flake => {
      const z = Math.random()
      return { x: Math.random() * w, y: top ? -10 - Math.random() * 40 : Math.random() * h, z, r: 0.8 + z * 1.8, vy: 18 + z * 42, amp: 6 + Math.random() * 14, ph: Math.random() * 6.28, f: 0.4 + Math.random() * 0.9, rot: Math.random() * 6.28 }
    }

    const size = () => {
      w = innerWidth
      h = o.header ? HEADER : innerHeight
      const dpr = devicePixelRatio || 1
      canvas.width = w * dpr
      canvas.height = h * dpr
      canvas.style.width = `${w}px`
      canvas.style.height = `${h}px`
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
      pile = new Float32Array(Math.ceil(w / BIN) + 1)
      const count = Math.round((o.density * w * h) / 100000 / (o.header ? 1 : 2))
      flakes = Array.from({ length: count }, () => spawn(false))
    }

    const settle = (i: number, r: number) => {
      for (let d = -2; d <= 2; d++) if (i + d >= 0 && i + d < pile.length) pile[i + d] = Math.min(PILE, pile[i + d] + r * 0.35 * (1 - Math.abs(d) * 0.3))
    }

    const draw = (dt: number, t: number) => {
      const floor = h - STATUS
      ctx.clearRect(0, 0, w, h)
      ctx.fillStyle = ctx.strokeStyle = '#ffffff'
      ctx.lineCap = 'round'
      for (const f of flakes) {
        f.y += f.vy * o.speed * dt
        f.x += o.wind * 30 * (0.5 + f.z) * dt
        f.rot += dt * 0.6
        const x = f.x + Math.sin(t * f.f + f.ph) * f.amp
        const r = f.r * o.size
        if (o.pile) {
          const i = Math.max(0, Math.min(pile.length - 1, Math.round(x / BIN)))
          if (f.y + r >= floor - pile[i]) {
            if (f.z > 0.45) settle(i, r)
            Object.assign(f, spawn(true))
            continue
          }
        }
        if (f.y > h + 10 || x < -30 || x > w + 30) {
          Object.assign(f, spawn(true))
          if (Math.abs(o.wind) > 0.05 && Math.random() < 0.5) Object.assign(f, { x: o.wind > 0 ? -20 : w + 20, y: Math.random() * h * 0.6 })
          continue
        }
        ctx.globalAlpha = (0.35 + f.z * 0.55) * (o.header ? Math.max(0, Math.min(1, (h - 6 - f.y) / 22)) : 1)
        ctx.beginPath()
        if (o.crystals && r > 1.6) {
          const s = r * 1.9
          ctx.lineWidth = Math.max(0.8, s / 6)
          for (let k = 0; k < 3; k++) {
            const dx = Math.cos(f.rot + (k * Math.PI) / 3) * s
            const dy = Math.sin(f.rot + (k * Math.PI) / 3) * s
            ctx.moveTo(x - dx, f.y - dy)
            ctx.lineTo(x + dx, f.y + dy)
          }
          ctx.stroke()
        } else {
          ctx.arc(x, f.y, r, 0, 6.283)
          ctx.fill()
        }
      }
      if (o.pile) {
        ctx.globalAlpha = 0.92
        ctx.beginPath()
        ctx.moveTo(0, floor)
        for (let i = 0; i < pile.length; i++) ctx.lineTo(i * BIN, floor - pile[i])
        ctx.lineTo(w, floor)
        ctx.fill()
      }
      ctx.globalAlpha = 1
    }

    const tick = (now: number) => {
      frame = requestAnimationFrame(tick)
      draw(Math.min(0.05, (now - last) / 1000), now / 1000)
      last = now
    }
    const run = () => {
      cancelAnimationFrame(frame)
      if (document.hidden) return
      last = performance.now()
      frame = requestAnimationFrame(tick)
    }

    size()
    run()
    addEventListener('resize', size)
    document.addEventListener('visibilitychange', run)
    return () => {
      cancelAnimationFrame(frame)
      removeEventListener('resize', size)
      document.removeEventListener('visibilitychange', run)
    }
  })
</script>

<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
  canvas {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 45;
    pointer-events: none;
  }
</style>
