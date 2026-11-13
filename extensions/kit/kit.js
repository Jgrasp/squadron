;(() => {
  const listeners = []
  const watchers = []
  let values = null
  const post = (data) => parent.postMessage(data, '*')
  window.squadron = {
    send: (message) => post({ squadron: 'extension', message }),
    on: (listener) => listeners.push(listener),
    options: (watcher) => {
      watchers.push(watcher)
      if (values) watcher(values)
    },
    setOption: (id, value) => post({ squadron: 'option', id, value }),
  }
  addEventListener('message', (e) => {
    if (e.source !== parent) return
    const data = e.data ?? {}
    if (data.squadron === 'theme') for (const [name, value] of Object.entries(data.vars)) document.documentElement.style.setProperty(`--${name}`, value)
    if (data.squadron === 'extension') for (const listener of listeners) listener(data.message)
    if (data.squadron === 'options') {
      values = data.values
      for (const watcher of watchers) watcher(values)
    }
  })
  new ResizeObserver(() => post({ squadron: 'height', height: document.documentElement.scrollHeight })).observe(document.documentElement)
})()
