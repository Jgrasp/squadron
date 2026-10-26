let context: AudioContext | undefined

export function chime() {
  context ??= new AudioContext()
  context.resume()
  const at = context.currentTime
  for (const [i, frequency] of [880, 1318.5].entries()) {
    const start = at + i * 0.12
    const tone = context.createOscillator()
    const gain = context.createGain()
    tone.frequency.value = frequency
    gain.gain.setValueAtTime(0, start)
    gain.gain.linearRampToValueAtTime(0.15, start + 0.01)
    gain.gain.exponentialRampToValueAtTime(0.001, start + 0.6)
    tone.connect(gain).connect(context.destination)
    tone.start(start)
    tone.stop(start + 0.6)
  }
}
