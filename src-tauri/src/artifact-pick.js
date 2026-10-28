;(() => {
  let on = false
  let box = null
  let current = null

  const quote = (s) => (/^[A-Za-z_][\w-]*$/.test(s) ? s : CSS.escape(s))

  function selector(el) {
    const parts = []
    for (let node = el; node && node.nodeType === 1 && node !== document.documentElement; node = node.parentElement) {
      if (node.id) {
        parts.unshift(`#${quote(node.id)}`)
        break
      }
      let part = node.localName
      const classes = [...node.classList].filter((c) => !c.startsWith('squadron-')).slice(0, 2)
      if (classes.length) part += classes.map((c) => `.${quote(c)}`).join('')
      const same = node.parentElement ? [...node.parentElement.children].filter((c) => c.localName === node.localName) : []
      if (same.length > 1) part += `:nth-of-type(${same.indexOf(node) + 1})`
      parts.unshift(part)
      if (node === document.body || parts.length >= 5) break
    }
    return parts.join(' > ')
  }

  function frame(el) {
    if (!box) {
      box = document.createElement('div')
      box.style.cssText = 'position:fixed;z-index:2147483647;pointer-events:none;border:2px solid #e8875a;background:rgba(232,135,90,.12);border-radius:3px;transition:all 60ms'
      document.documentElement.appendChild(box)
    }
    const r = el.getBoundingClientRect()
    Object.assign(box.style, { display: 'block', left: `${r.left - 2}px`, top: `${r.top - 2}px`, width: `${r.width + 4}px`, height: `${r.height + 4}px` })
  }

  function stop(tell) {
    on = false
    current = null
    if (box) box.style.display = 'none'
    document.documentElement.style.cursor = ''
    if (tell) parent.postMessage({ squadron: 'pick-off' }, '*')
  }

  addEventListener('message', (e) => {
    if (e.source !== parent || e.data?.squadron !== 'pick') return
    if (e.data.on) {
      on = true
      document.documentElement.style.cursor = 'crosshair'
    } else stop(false)
  })

  addEventListener(
    'mouseover',
    (e) => {
      if (!on || !(e.target instanceof Element) || e.target === box) return
      current = e.target
      frame(current)
    },
    true,
  )

  for (const type of ['mousedown', 'mouseup', 'click', 'dblclick', 'submit']) {
    addEventListener(
      type,
      (e) => {
        if (!on) return
        e.preventDefault()
        e.stopImmediatePropagation()
        if (type !== 'click' || !current) return
        const html = current.outerHTML
        parent.postMessage(
          {
            squadron: 'picked',
            selector: selector(current),
            html: html.length > 3000 ? `${html.slice(0, 3000)}…` : html,
            text: (current.innerText || '').trim().slice(0, 200),
            page: decodeURIComponent(location.pathname.split('/').slice(3).join('/')),
          },
          '*',
        )
        stop(false)
      },
      true,
    )
  }

  addEventListener('keydown', (e) => {
    if (on && e.key === 'Escape') stop(true)
  })
})()
