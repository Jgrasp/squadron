;(() => {
  const listeners = []
  window.squadron = {
    send: (message) => parent.postMessage({ squadron: 'extension', message }, '*'),
    on: (listener) => listeners.push(listener),
  }
  addEventListener('message', (e) => {
    if (e.source !== parent) return
    const data = e.data ?? {}
    if (data.squadron === 'theme') for (const [name, value] of Object.entries(data.vars)) document.documentElement.style.setProperty(`--${name}`, value)
    if (data.squadron === 'mod') for (const listener of listeners) listener(data.message)
  })
})()
