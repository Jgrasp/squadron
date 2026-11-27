import type { EffectKind } from './extensions'

type Options = Record<string, any>

export type Scene = { w: number; h: number; o: Options; color: (name: string) => string; mono: string }

export type Draw = (ctx: CanvasRenderingContext2D, dt: number, t: number) => void

const STATUS = 26

const rnd = (a: number, b: number) => a + Math.random() * (b - a)

const dot = (ctx: CanvasRenderingContext2D, x: number, y: number, r: number) => {
  ctx.beginPath()
  ctx.arc(x, y, r, 0, 6.283)
  ctx.fill()
}

export const heightOf = (o: Options, window: number) => (o.area === 'header' ? 54 : o.area === 'top' ? 180 : window)

function snow({ w, h, o }: Scene): Draw {
  const BIN = 4
  const header = o.area === 'header'
  const pile = new Float32Array(Math.ceil(w / BIN) + 1)
  const spawn = (top: boolean) => {
    const z = Math.random()
    return { x: rnd(0, w), y: top ? rnd(-50, -10) : rnd(0, h), z, r: 0.8 + z * 1.8, vy: 18 + z * 42, amp: rnd(6, 20), ph: rnd(0, 6.28), f: rnd(0.4, 1.3), rot: rnd(0, 6.28) }
  }
  const flakes = Array.from({ length: Math.round((o.density * w * h) / 100000 / (header ? 1 : 2)) }, () => spawn(false))
  const floor = h - STATUS
  return (ctx, dt, t) => {
    ctx.fillStyle = ctx.strokeStyle = '#ffffff'
    ctx.lineCap = 'round'
    for (const f of flakes) {
      f.y += f.vy * o.speed * dt
      f.x += o.wind * 30 * (0.5 + f.z) * dt
      f.rot += dt * 0.6
      const x = f.x + Math.sin(t * f.f + f.ph) * f.amp
      const r = f.r * o.size
      if (o.pile && !header) {
        const i = Math.max(0, Math.min(pile.length - 1, Math.round(x / BIN)))
        if (f.y + r >= floor - pile[i]) {
          if (f.z > 0.45) for (let d = -2; d <= 2; d++) if (i + d >= 0 && i + d < pile.length) pile[i + d] = Math.min(7, pile[i + d] + r * 0.35 * (1 - Math.abs(d) * 0.3))
          Object.assign(f, spawn(true))
          continue
        }
      }
      if (f.y > h + 10 || x < -30 || x > w + 30) {
        Object.assign(f, spawn(true))
        if (Math.abs(o.wind) > 0.05 && Math.random() < 0.5) Object.assign(f, { x: o.wind > 0 ? -20 : w + 20, y: rnd(0, h * 0.6) })
        continue
      }
      ctx.globalAlpha = (0.35 + f.z * 0.55) * (header ? Math.max(0, Math.min(1, (h - 6 - f.y) / 22)) : 1)
      if (o.shape === 'crystals' && r > 1.6) {
        const s = r * 1.9
        ctx.lineWidth = Math.max(0.8, s / 6)
        ctx.beginPath()
        for (let k = 0; k < 3; k++) {
          const dx = Math.cos(f.rot + (k * Math.PI) / 3) * s
          const dy = Math.sin(f.rot + (k * Math.PI) / 3) * s
          ctx.moveTo(x - dx, f.y - dy)
          ctx.lineTo(x + dx, f.y + dy)
        }
        ctx.stroke()
      } else dot(ctx, x, f.y, r)
    }
    if (o.pile && !header) {
      ctx.globalAlpha = 0.92
      ctx.beginPath()
      ctx.moveTo(0, floor)
      for (let i = 0; i < pile.length; i++) ctx.lineTo(i * BIN, floor - pile[i])
      ctx.lineTo(w, floor)
      ctx.fill()
    }
  }
}

const GLYPHS = 'ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝ0123456789:=*+<>'

function matrix({ w, h, o, color, mono }: Scene): Draw {
  const glyph = () => GLYPHS[(Math.random() * GLYPHS.length) | 0]
  const rows = Math.ceil(h / o.size)
  const [head, body] = o.color === 'accent' ? [color('--accent-soft'), color('--accent')] : o.color === 'ok' ? [color('--ok-text'), color('--ok')] : ['#7dffa0', '#2fd16b']
  const drop = (i: number, anywhere: boolean) => ({ x: i * o.size, y: anywhere ? rnd(0, rows) : -rnd(0, rows / 2), v: rnd(6, 16), cells: Array.from({ length: 64 }, glyph) })
  const cols = Array.from({ length: Math.floor(w / o.size) }, (_, i) => i)
    .filter(() => Math.random() < o.density)
    .map((i) => drop(i, true))
  return (ctx, dt) => {
    ctx.font = `${o.size}px ${mono}`
    ctx.textBaseline = 'top'
    for (const c of cols) {
      c.y += c.v * o.speed * dt
      if (Math.random() < 0.08) c.cells[(Math.random() * 64) | 0] = glyph()
      const top = Math.floor(c.y)
      if (top - o.trail > rows) {
        Object.assign(c, drop(c.x / o.size, false))
        continue
      }
      for (let k = 0; k < o.trail; k++) {
        const row = top - k
        if (row < 0 || row > rows) continue
        ctx.globalAlpha = k === 0 ? Math.min(1, o.opacity * 2.2) : o.opacity * (1 - k / o.trail)
        ctx.fillStyle = k === 0 ? head : body
        ctx.fillText(c.cells[row % 64], c.x, row * o.size)
      }
    }
  }
}

function fireflies({ w, h, o, color }: Scene): Draw {
  const c = o.color === 'accent' ? color('--accent') : o.color === 'green' ? '#9ef0b4' : '#f4dc8a'
  const flies = Array.from({ length: Math.round((o.density * w * h) / 1e6) }, () => ({ x: rnd(0, w), y: rnd(0, h), a: rnd(0, 6.28), f: rnd(0.4, 1.2), ph: rnd(0, 6.28), r: rnd(1.2, 2.4) }))
  return (ctx, dt, t) => {
    ctx.fillStyle = c
    for (const p of flies) {
      p.a += rnd(-1.2, 1.2) * dt
      p.x = (p.x + Math.cos(p.a) * 14 * o.speed * dt + w) % w
      p.y = (p.y + Math.sin(p.a) * 10 * o.speed * dt + h) % h
      const glow = Math.max(0, Math.sin(t * p.f + p.ph))
      const r = p.r * o.size
      ctx.globalAlpha = 0.12 * glow
      dot(ctx, p.x, p.y, r * 5)
      ctx.globalAlpha = 0.25 + 0.7 * glow
      dot(ctx, p.x, p.y, r)
    }
  }
}

function embers({ w, h, o, color }: Scene): Draw {
  const c = o.color === 'soft' ? color('--accent-soft') : o.color === 'gold' ? '#f4dc8a' : color('--accent')
  const spawn = (anywhere: boolean) => {
    const life = rnd(3, 7)
    return { x: rnd(0, w), life, age: anywhere ? rnd(0, life) : 0, vy: rnd(30, 70), ph: rnd(0, 6.28), f: rnd(1, 3), r: rnd(0.8, 2) }
  }
  const sparks = Array.from({ length: Math.round((o.density * w) / 1000) }, () => spawn(true))
  return (ctx, dt, t) => {
    ctx.fillStyle = c
    for (const p of sparks) {
      p.age += dt * o.speed
      if (p.age > p.life) {
        Object.assign(p, spawn(false))
        continue
      }
      const k = p.age / p.life
      const x = p.x + Math.sin(p.age * p.f + p.ph) * 18 * k
      const y = h - STATUS - p.vy * p.age
      const flicker = 0.6 + 0.4 * Math.sin(t * 13 + p.ph)
      const r = p.r * o.size * (1 - k * 0.6)
      ctx.globalAlpha = 0.18 * (1 - k) * flicker
      dot(ctx, x, y, r * 3.5)
      ctx.globalAlpha = (1 - k) * flicker
      dot(ctx, x, y, r)
    }
  }
}

function stars({ w, h, o, color }: Scene): Draw {
  const c = o.color === 'blue' ? color('--remote-text') : o.color === 'soft' ? color('--accent-soft') : '#ffffff'
  const header = o.area === 'header'
  const sky = Array.from({ length: Math.round((o.density * w * h) / 1e5 / (header ? 1 : 6)) }, () => ({ x: rnd(0, w), y: rnd(0, h), r: rnd(0.4, 1.3), f: rnd(0.5, 2), ph: rnd(0, 6.28) }))
  let shot: { x: number; y: number; age: number } | null = null
  let next = rnd(1, 3)
  const drop = header ? 30 : 120
  const len = Math.hypot(260, drop)
  return (ctx, dt, t) => {
    ctx.fillStyle = c
    for (const p of sky) {
      ctx.globalAlpha = 0.25 + 0.6 * (0.5 + 0.5 * Math.sin(t * p.f * o.twinkle + p.ph))
      dot(ctx, p.x, p.y, p.r)
    }
    next -= dt
    if (!shot && o.shooting > 0 && next <= 0) {
      shot = { x: rnd(w * 0.1, w * 0.7), y: rnd(0, h * 0.4), age: 0 }
      next = rnd(30, 90) / o.shooting
    }
    if (!shot) return
    shot.age += dt
    const k = shot.age / 0.9
    if (k >= 1) {
      shot = null
      return
    }
    const hx = shot.x + k * 260
    const hy = shot.y + k * drop
    const tx = hx - (90 * 260) / len
    const ty = hy - (90 * drop) / len
    const g = ctx.createLinearGradient(tx, ty, hx, hy)
    g.addColorStop(0, 'rgba(255,255,255,0)')
    g.addColorStop(1, 'rgba(255,255,255,0.95)')
    ctx.globalAlpha = 1 - k
    ctx.strokeStyle = g
    ctx.lineWidth = 1.5
    ctx.lineCap = 'round'
    ctx.beginPath()
    ctx.moveTo(tx, ty)
    ctx.lineTo(hx, hy)
    ctx.stroke()
  }
}

function rain({ w, h, o, color }: Scene): Draw {
  const c = o.color === 'white' ? '#ffffff' : o.color === 'soft' ? color('--accent-soft') : color('--remote-text')
  const floor = h - STATUS
  const vx = o.wind * 0.35
  const spawn = (anywhere: boolean) => {
    const z = Math.random()
    return { x: rnd(-w * 0.3, w * 1.3), y: anywhere ? rnd(0, h) : rnd(-80, -10), z, v: 500 + z * 400 }
  }
  const drops = Array.from({ length: Math.round((o.density * w * h) / 2e5) }, () => spawn(true))
  let rings: { x: number; age: number }[] = []
  return (ctx, dt) => {
    ctx.strokeStyle = c
    ctx.lineCap = 'round'
    for (const p of drops) {
      const d = p.v * o.speed * dt
      p.y += d
      p.x += d * vx
      if (p.y >= floor) {
        if (o.splash && p.z > 0.4) rings.push({ x: p.x, age: 0 })
        Object.assign(p, spawn(false))
        continue
      }
      const l = o.length * (0.5 + p.z)
      ctx.globalAlpha = 0.15 + p.z * 0.35
      ctx.lineWidth = 0.6 + p.z * 0.8
      ctx.beginPath()
      ctx.moveTo(p.x - l * vx, p.y - l)
      ctx.lineTo(p.x, p.y)
      ctx.stroke()
    }
    ctx.lineWidth = 1
    rings = rings.filter((r) => (r.age += dt) < 0.35)
    for (const r of rings) {
      const k = r.age / 0.35
      ctx.globalAlpha = 0.6 * (1 - k)
      ctx.beginPath()
      ctx.ellipse(r.x, floor, 2 + k * 9, 1 + k * 2.5, 0, Math.PI, 0)
      ctx.stroke()
    }
  }
}

function aurora({ w, h, o, color }: Scene): Draw {
  const colors = o.color === 'theme' ? [color('--ok'), color('--accent'), color('--advisor')] : ['#5ff0b0', color('--remote-dot'), color('--advisor')]
  return (ctx, _dt, t) => {
    ctx.globalCompositeOperation = 'lighter'
    for (let b = 0; b < o.bands; b++) {
      const c = colors[b % colors.length]
      const T = t * 0.25 * o.speed + b * 1.7
      const g = ctx.createLinearGradient(0, 0, 0, h)
      g.addColorStop(0, 'transparent')
      g.addColorStop(0.45, c)
      g.addColorStop(1, 'transparent')
      ctx.fillStyle = g
      ctx.globalAlpha = o.intensity * (0.6 + 0.4 * Math.sin(T * 0.7))
      const mid = (x: number) => h * (0.45 + 0.22 * Math.sin(x / 260 + T) + 0.1 * Math.sin(x / 90 - T * 1.6))
      const thick = (x: number) => h * (0.18 + 0.12 * Math.sin(x / 180 - T * 1.3 + b))
      ctx.beginPath()
      ctx.moveTo(0, mid(0) - thick(0))
      for (let x = 0; x <= w; x += 16) ctx.lineTo(x, mid(x) - thick(x))
      for (let x = w; x >= 0; x -= 16) ctx.lineTo(x, mid(x) + thick(x))
      ctx.fill()
    }
    ctx.globalCompositeOperation = 'source-over'
  }
}

export const DRAWS: Record<EffectKind, (scene: Scene) => Draw> = { snow, matrix, fireflies, embers, stars, rain, aurora }
