<script lang="ts">
  import { DRAWS, heightOf, type Draw } from './effects'
  import { effectOptions, type Effect } from './extensions'

  let { fx }: { fx: Effect } = $props()

  let canvas: HTMLCanvasElement

  $effect(() => {
    const o = effectOptions(fx)
    const ctx = canvas.getContext('2d')!
    const style = getComputedStyle(document.documentElement)
    let paint: Draw = () => {}
    let w = 0
    let h = 0
    let frame = 0
    let last = 0

    const size = () => {
      w = innerWidth
      h = heightOf(o, innerHeight)
      const dpr = devicePixelRatio || 1
      canvas.width = w * dpr
      canvas.height = h * dpr
      canvas.style.width = `${w}px`
      canvas.style.height = `${h}px`
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
      paint = DRAWS[fx.kind]({ w, h, o, color: (name) => style.getPropertyValue(name).trim(), mono: style.getPropertyValue('--mono') || 'monospace' })
    }

    const tick = (now: number) => {
      frame = requestAnimationFrame(tick)
      ctx.clearRect(0, 0, w, h)
      paint(ctx, Math.min(0.05, (now - last) / 1000), now / 1000)
      ctx.globalAlpha = 1
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
