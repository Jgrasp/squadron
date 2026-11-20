;(() => {
  if (globalThis.__sq) return
  const S = { ids: new WeakMap(), els: new Map(), n: 0 }

  const SKIP = new Set(['script', 'style', 'noscript', 'template', 'head', 'meta', 'link', 'br', 'wbr'])
  const FIELDS = new Set(['textbox', 'searchbox', 'combobox', 'spinbutton', 'slider'])
  const ACTIVE = new Set([
    'link', 'button', 'textbox', 'searchbox', 'checkbox', 'radio', 'combobox', 'listbox', 'slider', 'spinbutton',
    'switch', 'tab', 'menuitem', 'menuitemcheckbox', 'menuitemradio', 'option', 'treeitem', 'file', 'clickable',
  ])
  const GROUPS = new Set(['navigation', 'main', 'form', 'dialog', 'alertdialog', 'complementary', 'search', 'banner', 'contentinfo', 'tablist', 'menu', 'menubar', 'listbox', 'tree', 'grid'])
  const LIVE = new Set(['alert', 'status'])
  const INPUTS = {
    button: 'button', submit: 'button', reset: 'button', image: 'button', checkbox: 'checkbox', radio: 'radio',
    range: 'slider', number: 'spinbutton', search: 'searchbox', file: 'file', hidden: '', color: 'button',
  }
  const LANDMARKS = { nav: 'navigation', main: 'main', form: 'form', dialog: 'dialog', aside: 'complementary', search: 'search' }

  const clip = (s, n) => {
    s = (s || '').replace(/\s+/g, ' ').trim()
    return s.length > n ? `${s.slice(0, n - 1)}…` : s
  }
  const quote = (s) => JSON.stringify(s)

  function ref(e) {
    let r = S.ids.get(e)
    if (!r) {
      r = `e${++S.n}`
      S.ids.set(e, r)
      S.els.set(r, new WeakRef(e))
    }
    return r
  }

  function el(r) {
    const e = S.els.get(r)?.deref()
    if (!e || !e.isConnected) throw new Error(`${r} is gone: the page changed, look again`)
    return e
  }

  const kids = (n) => (n.shadowRoot ? [...n.shadowRoot.children] : n.localName === 'slot' ? n.assignedElements({ flatten: true }) : [...n.children])

  function role(e) {
    const explicit = e.getAttribute('role')
    if (explicit) return explicit.trim().split(/\s+/)[0]
    const tag = e.localName
    if (tag === 'a' || tag === 'area') return e.hasAttribute('href') ? 'link' : ''
    if (tag === 'button' || tag === 'summary') return 'button'
    if (tag === 'select') return e.multiple || e.size > 1 ? 'listbox' : 'combobox'
    if (tag === 'textarea') return 'textbox'
    if (tag === 'input') {
      const type = (e.getAttribute('type') || 'text').toLowerCase()
      return type in INPUTS ? INPUTS[type] : 'textbox'
    }
    if (/^h[1-6]$/.test(tag)) return 'heading'
    if (tag in LANDMARKS) return LANDMARKS[tag]
    if (tag === 'iframe') return 'iframe'
    if (e.isContentEditable && e.hasAttribute('contenteditable')) return 'textbox'
    return ''
  }

  function text(e) {
    return e.innerText ?? e.textContent ?? ''
  }

  function labelText(l) {
    const copy = l.cloneNode(true)
    copy.querySelectorAll('select, textarea, input, button').forEach((c) => c.remove())
    return copy.textContent
  }

  function name(e, r) {
    const label = e.getAttribute('aria-label')
    if (label?.trim()) return label
    const by = e.getAttribute('aria-labelledby')
    if (by) {
      const found = by.split(/\s+/).map((id) => (document.getElementById(id) ? text(document.getElementById(id)) : '')).join(' ')
      if (found.trim()) return found
    }
    if (e.labels?.length) {
      const found = [...e.labels].map(labelText).join(' ')
      if (found.trim()) return found
    }
    if (e.localName === 'input' && ['button', 'submit', 'reset'].includes(e.type)) return e.value || e.type
    if (e.localName === 'img' || (e.localName === 'input' && e.type === 'image')) return e.alt || e.title || ''
    if (FIELDS.has(r) && e.localName !== 'select') return e.placeholder || e.title || ''
    if (e.localName === 'select') return e.title || ''
    const inner = text(e)
    if (inner.trim()) return inner
    const icon = e.querySelector('img[alt], [aria-label], svg title')
    return icon?.getAttribute('alt') || icon?.getAttribute('aria-label') || icon?.textContent || e.title || ''
  }

  function state(e, r) {
    const out = []
    if (r === 'checkbox' || r === 'radio' || r === 'switch') {
      const on = 'checked' in e && e.localName === 'input' ? e.checked : e.getAttribute('aria-checked') === 'true'
      out.push(on ? 'on' : 'off')
    } else if (e.localName === 'select') {
      out.push(`=${quote(clip(e.selectedOptions[0]?.text ?? '', 40))}`, `[${e.options.length} options]`)
    } else if (FIELDS.has(r) || r === 'textbox') {
      const value = e.isContentEditable ? text(e) : e.value
      if (value) out.push(e.type === 'password' ? '=••••' : `=${quote(clip(value, 40))}`)
    } else if (r === 'file' && e.files?.length) out.push(`=${e.files.length} file(s)`)
    if (e.disabled || e.getAttribute('aria-disabled') === 'true') out.push('disabled')
    const expanded = e.getAttribute('aria-expanded')
    if (expanded) out.push(expanded === 'true' ? 'expanded' : 'collapsed')
    if (e.getAttribute('aria-selected') === 'true' || e.getAttribute('aria-current')) out.push('current')
    if (e.required) out.push('required')
    if (e.getAttribute('aria-invalid') === 'true') out.push('invalid')
    if (document.activeElement === e) out.push('focused')
    return out.length ? ` ${out.join(' ')}` : ''
  }

  function line(e, r, depth) {
    const label = clip(name(e, r), 60)
    return `${'  '.repeat(depth)}${r === 'heading' ? `h${e.getAttribute('aria-level') || e.localName[1] || 2}` : r} ${ref(e)}${label ? ` ${quote(label)}` : ''}${state(e, r)}`
  }

  function inView(e) {
    const b = e.getBoundingClientRect()
    return b.bottom > 0 && b.right > 0 && b.top < innerHeight && b.left < innerWidth
  }

  function walk(n, depth, out, o, pointer) {
    let links = 0
    let folded = 0
    for (const e of kids(n)) {
      const tag = e.localName
      if (SKIP.has(tag) || e.getAttribute('aria-hidden') === 'true' || e.hasAttribute('inert')) continue
      const cs = getComputedStyle(e)
      if (cs.display === 'none' || cs.opacity === '0') continue
      if (cs.display === 'contents') {
        walk(e, depth, out, o, pointer)
        continue
      }
      const hidden = cs.visibility === 'hidden' || cs.visibility === 'collapse'
      let r = hidden ? '' : role(e)
      const clicky = cs.cursor === 'pointer'
      if (!r && !hidden && clicky && !pointer && (e.tabIndex >= 0 || e.childElementCount < 4) && text(e).trim()) r = 'clickable'
      if (r === 'iframe') {
        if (!o.view || inView(e)) out.push(`${'  '.repeat(depth)}iframe ${ref(e)} ${quote(clip(e.title || e.name || e.src, 60))}`)
        continue
      }
      if (ACTIVE.has(r) && !GROUPS.has(r)) {
        const box = e.getBoundingClientRect()
        if ((box.width || box.height || r === 'radio' || r === 'checkbox' || r === 'file') && (!o.view || inView(e))) {
          if (r === 'link' && ++links > 8) {
            folded++
            continue
          }
          out.push(line(e, r, depth))
        }
        continue
      }
      if (r === 'heading') {
        if (!o.view || inView(e)) out.push(line(e, r, depth))
        continue
      }
      if (LIVE.has(r) || e.getAttribute('aria-live') === 'assertive') {
        const said = clip(text(e), 120)
        if (said) out.push(`${'  '.repeat(depth)}${r || 'alert'} ${ref(e)} ${quote(said)}`)
        continue
      }
      if (GROUPS.has(r) && (r !== 'banner' || tag !== 'header' || !e.closest('main,article,section'))) {
        const start = out.length
        walk(e, depth + 1, out, o, clicky)
        if (out.length > start || r === 'dialog' || r === 'alertdialog') {
          const label = clip(e.getAttribute('aria-label') || (r === 'form' ? e.getAttribute('name') || '' : ''), 40)
          out.splice(start, 0, `${'  '.repeat(depth)}${r} ${ref(e)}${label ? ` ${quote(label)}` : ''}`)
        }
        continue
      }
      walk(e, depth, out, o, clicky || pointer)
    }
    if (folded) out.push(`${'  '.repeat(depth)}+${folded} links (look ref=${ref(n.host ?? n)})`)
  }

  function snapshot(o) {
    const root = o.ref ? el(o.ref) : document.body
    const out = []
    if (root) walk(root, 0, out, o, false)
    return { lines: out, n: S.n, title: document.title, url: location.href, scroll: [Math.round(scrollY), Math.max(0, document.documentElement.scrollHeight - innerHeight)] }
  }

  function owner(e) {
    for (let n = e; n && n !== document.body; n = n.parentElement) {
      const r = role(n)
      if (ACTIVE.has(r)) return [n, r]
    }
    return [e, '']
  }

  function find(o) {
    const wanted = o.query.toLowerCase()
    const seen = new Set()
    const out = []
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT)
    for (let node = walker.nextNode(); node && out.length < 20; node = walker.nextNode()) {
      let hit
      if (node.nodeType === 3) {
        if (node.data.toLowerCase().includes(wanted)) hit = node.parentElement
      } else if (['aria-label', 'placeholder', 'title', 'alt', 'value', 'name'].some((a) => node.getAttribute(a)?.toLowerCase().includes(wanted))) hit = node
      if (!hit || !hit.checkVisibility?.()) continue
      const [e, r] = owner(hit)
      if (seen.has(e)) continue
      seen.add(e)
      out.push(r ? line(e, r, 0) : `${e.localName} ${ref(e)} ${quote(clip(text(e), 80))}`)
    }
    return { lines: out, n: S.n }
  }

  const BLOCKS = /^(block|flex|grid|list-item|table|table-row|table-row-group|table-header-group|table-footer-group|flow-root)$/

  function read(o) {
    const root = o.ref ? el(o.ref) : document.querySelector('main, [role=main], article') || document.body
    const out = []
    const push = (s) => {
      s = s.replace(/[ \t]+/g, ' ').trim()
      if (s) out.push(s)
    }
    function visit(e) {
      const tag = e.localName
      if (SKIP.has(tag) || e.getAttribute('aria-hidden') === 'true') return
      const cs = getComputedStyle(e)
      if (cs.display === 'none' || cs.visibility === 'hidden') return
      if (/^h[1-6]$/.test(tag)) return push(`${'#'.repeat(+tag[1])} ${text(e)}`)
      if (tag === 'tr') return push(`| ${[...e.cells].map((c) => clip(text(c), 60)).join(' | ')} |`)
      if (tag === 'pre') return out.push(`\`\`\`\n${text(e).trimEnd()}\n\`\`\``)
      if (tag === 'img') return e.alt && push(`[image: ${e.alt}]`)
      if (tag === 'input' || tag === 'select' || tag === 'textarea') return
      const nested = [...e.children].some((c) => BLOCKS.test(getComputedStyle(c).display))
      if (!nested) return push(`${tag === 'li' ? '- ' : ''}${text(e)}`)
      let inline = ''
      for (const node of e.childNodes) {
        if (node.nodeType === 3) inline += node.data
        else if (node.nodeType === 1 && !BLOCKS.test(getComputedStyle(node).display)) inline += text(node)
        else {
          push(`${tag === 'li' && !out.length ? '- ' : ''}${inline}`)
          inline = ''
          visit(node)
        }
      }
      push(inline)
    }
    if (root) visit(root)
    const all = out.join('\n')
    const from = o.offset || 0
    const size = o.limit || 6000
    const part = all.slice(from, from + size)
    const more = all.length - from - part.length
    return { text: part + (more > 0 ? `\n… ${more} more characters: read offset=${from + part.length}` : ''), n: S.n }
  }

  function selector(e) {
    const esc = (s) => (/^[A-Za-z_][\w-]*$/.test(s) ? s : CSS.escape(s))
    const parts = []
    for (let node = e; node && node.nodeType === 1 && node !== document.documentElement; node = node.parentElement) {
      if (node.id) {
        parts.unshift(`#${esc(node.id)}`)
        break
      }
      let part = node.localName
      const classes = [...node.classList].slice(0, 2)
      if (classes.length) part += classes.map((c) => `.${esc(c)}`).join('')
      const same = node.parentElement ? [...node.parentElement.children].filter((c) => c.localName === node.localName) : []
      if (same.length > 1) part += `:nth-of-type(${same.indexOf(node) + 1})`
      parts.unshift(part)
      if (node === document.body || parts.length >= 5) break
    }
    return parts.join(' > ')
  }

  function pierce(x, y) {
    let e = document.elementFromPoint(x, y)
    while (e?.shadowRoot) {
      const inner = e.shadowRoot.elementFromPoint(x, y)
      if (!inner || inner === e) break
      e = inner
    }
    return e
  }

  function describe(e) {
    const r = role(e)
    const b = e.getBoundingClientRect()
    const html = e.outerHTML
    return {
      ref: ref(e),
      selector: selector(e),
      role: r || e.localName,
      name: clip(name(e, r), 80),
      html: html.length > 1500 ? `${html.slice(0, 1500)}…` : html,
      rect: [b.x, b.y, b.width, b.height],
      n: S.n,
    }
  }

  function point(o) {
    const e = pierce(o.x, o.y)
    if (!e || e === document.documentElement || e === document.body) return null
    const b = e.getBoundingClientRect()
    return b.width * b.height < 0.9 * innerWidth * innerHeight ? describe(e) : null
  }

  function area(o) {
    const pad = Math.max(o.w, o.h) * 0.1
    const inside = (n) => {
      const b = n.getBoundingClientRect()
      const w = Math.max(0, Math.min(b.right, o.x + o.w + pad) - Math.max(b.left, o.x - pad))
      const h = Math.max(0, Math.min(b.bottom, o.y + o.h + pad) - Math.max(b.top, o.y - pad))
      return b.width * b.height ? (w * h) / (b.width * b.height) : 0
    }
    const hits = new Set()
    for (let i = 1; i < 6; i++) for (let j = 1; j < 6; j++) {
      const hit = pierce(o.x + (o.w * i) / 6, o.y + (o.h * j) / 6)
      if (hit && inside(hit) > 0.5) hits.add(hit)
    }
    let e = [...hits].reduce((a, b) => {
      let c = a
      while (c && !c.contains(b)) c = c.parentElement
      return c ?? a
    }, [...hits][0] ?? pierce(o.x + o.w / 2, o.y + o.h / 2))
    if (!e) return null
    while (e.parentElement && e.parentElement !== document.body && inside(e.parentElement) > 0.8) e = e.parentElement
    return describe(e)
  }

  function aim(r) {
    let e = el(r)
    let b = e.getBoundingClientRect()
    if ((!b.width || !b.height) && e.labels?.length) e = e.labels[0]
    e.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' })
    b = e.getBoundingClientRect()
    if (!b.width || !b.height) throw new Error(`${r} is not visible`)
    const x = b.x + b.width / 2
    const y = b.y + b.height / 2
    const hit = pierce(x, y)
    const target = el(r)
    const fine = !hit || e.contains(hit) || hit.contains(e) || target.contains(hit) || (target.labels && [...target.labels].some((l) => l.contains(hit)))
    if (!fine) {
      const [over, overRole] = owner(hit)
      return { x, y, covered: overRole ? line(over, overRole, 0) : `${over.localName} ${ref(over)} ${quote(clip(text(over), 60))}` }
    }
    return { x, y }
  }

  const NATIVE = new Set(['date', 'time', 'datetime-local', 'month', 'week', 'color', 'range'])

  function focus(o) {
    const e = el(o.ref)
    e.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' })
    if (e.localName === 'input' && NATIVE.has(e.type)) {
      const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set
      set.call(e, o.text)
      e.dispatchEvent(new Event('input', { bubbles: true }))
      e.dispatchEvent(new Event('change', { bubbles: true }))
      return { done: true }
    }
    e.focus()
    if (typeof e.select === 'function') e.select()
    else {
      const range = document.createRange()
      range.selectNodeContents(e)
      getSelection().removeAllRanges()
      getSelection().addRange(range)
    }
    return { done: false, empty: !(e.isContentEditable ? text(e) : e.value) }
  }

  function choose(o) {
    const e = el(o.ref)
    if (e.localName !== 'select') throw new Error(`${o.ref} is not a native select: click it, then click the option`)
    const wanted = o.text.trim().toLowerCase()
    const options = [...e.options]
    const opt =
      options.find((x) => x.value === o.text || x.text.trim().toLowerCase() === wanted) ?? options.find((x) => x.text.toLowerCase().includes(wanted))
    if (!opt) throw new Error(`no option "${o.text}" in ${o.ref}: ${options.slice(0, 12).map((x) => x.text.trim()).join(', ')}`)
    e.value = opt.value
    e.dispatchEvent(new Event('input', { bubbles: true }))
    e.dispatchEvent(new Event('change', { bubbles: true }))
    return opt.text.trim()
  }

  function checked(o) {
    const e = el(o.ref)
    return 'checked' in e && e.localName === 'input' ? e.checked : e.getAttribute('aria-checked') === 'true'
  }

  function rect(r) {
    const e = el(r)
    e.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' })
    const b = e.getBoundingClientRect()
    return [b.x, b.y, b.width, b.height]
  }

  function scroll(o) {
    if (o.ref) el(o.ref).scrollIntoView({ block: 'center', behavior: 'instant' })
    else scrollBy(0, o.y ?? innerHeight * 0.8)
    return Math.round(scrollY)
  }

  const POINTER = { pointerId: 1, pointerType: 'mouse', isPrimary: true }
  const FOCUSABLE = 'input, select, textarea, button, a[href], [tabindex], [contenteditable]:not([contenteditable=false])'

  function click(o) {
    const a = aim(o.ref)
    if (a.covered) return a
    const t = pierce(a.x, a.y) ?? el(o.ref)
    return new Promise((done) =>
      setTimeout(() => {
        const base = { bubbles: true, cancelable: true, composed: true, view: window, clientX: a.x, clientY: a.y }
        const fire = (type, C, extra) => t.dispatchEvent(new C(type, { ...base, ...extra }))
        fire('pointerover', PointerEvent, POINTER)
        fire('pointerenter', PointerEvent, { ...POINTER, bubbles: false })
        fire('mouseover', MouseEvent)
        fire('mouseenter', MouseEvent, { bubbles: false })
        fire('pointermove', PointerEvent, POINTER)
        fire('mousemove', MouseEvent)
        for (let i = 1; i <= o.clicks; i++) {
          const down = { button: 0, buttons: 1, detail: i }
          fire('pointerdown', PointerEvent, { ...POINTER, ...down })
          if (fire('mousedown', MouseEvent, down)) {
            const f = t.closest(FOCUSABLE)
            if (f) f.focus({ preventScroll: true })
            else document.activeElement?.blur()
          }
          const up = { button: 0, buttons: 0, detail: i }
          fire('pointerup', PointerEvent, { ...POINTER, ...up })
          fire('mouseup', MouseEvent, up)
          fire('click', MouseEvent, up)
        }
        if (o.clicks === 2) fire('dblclick', MouseEvent, { button: 0, detail: 2 })
        done({})
      }),
    )
  }

  const BUTTONS = new Set(['checkbox', 'radio', 'button', 'submit', 'reset', 'image'])

  function editable(e) {
    return !!e && (e.isContentEditable || e.localName === 'textarea' || (e.localName === 'input' && !BUTTONS.has(e.type) && !NATIVE.has(e.type) && e.type !== 'file'))
  }

  function insert(o) {
    const t = document.activeElement
    if (document.execCommand('insertText', false, o.text)) return {}
    if (!t || !('value' in t)) throw new Error('nothing focused takes text')
    Object.getOwnPropertyDescriptor(Object.getPrototypeOf(t), 'value')?.set?.call(t, o.text)
    t.dispatchEvent(new Event('input', { bubbles: true }))
    t.dispatchEvent(new Event('change', { bubbles: true }))
    return {}
  }

  function press(o) {
    const t = document.activeElement ?? document.body
    const m = o.modifiers
    const init = { key: o.key, code: o.code, keyCode: o.vk, which: o.vk, bubbles: true, cancelable: true, composed: true, altKey: !!(m & 1), ctrlKey: !!(m & 2), metaKey: !!(m & 4), shiftKey: !!(m & 8) }
    let go = t.dispatchEvent(new KeyboardEvent('keydown', init))
    if (go && o.text) go = t.dispatchEvent(new KeyboardEvent('keypress', { ...init, charCode: o.text.charCodeAt(0) }))
    if (go) perform(t, o, m)
    t.dispatchEvent(new KeyboardEvent('keyup', init))
    return {}
  }

  function perform(t, o, m) {
    if (m & 6) {
      if (o.key.toLowerCase() === 'a') typeof t.select === 'function' ? t.select() : document.execCommand('selectAll')
      return
    }
    const step = m & 8 ? -1 : 1
    switch (o.key) {
      case 'Enter':
        if (t.localName === 'textarea') return document.execCommand('insertLineBreak')
        if (t.isContentEditable) return document.execCommand('insertParagraph')
        if (t.localName === 'input' && t.form) {
          const submit = t.form.querySelector('button:not([type]), button[type=submit], input[type=submit], input[type=image]')
          return submit ? submit.click() : t.form.requestSubmit()
        }
        if (t.localName === 'a' || t.localName === 'button' || t.getAttribute('role') === 'button') t.click()
        return
      case ' ':
        if (editable(t)) return document.execCommand('insertText', false, ' ')
        if (t.localName === 'button' || (t.localName === 'input' && BUTTONS.has(t.type)) || t.getAttribute('role') === 'button') return t.click()
        return scrollBy(0, step * innerHeight * 0.8)
      case 'Tab': {
        const all = [...document.querySelectorAll(FOCUSABLE)].filter((e) => e.tabIndex >= 0 && !e.disabled && e.checkVisibility())
        const i = all.indexOf(t)
        all[i < 0 ? (step > 0 ? 0 : all.length - 1) : (i + step + all.length) % all.length]?.focus()
        return
      }
      case 'Backspace':
        if (editable(t)) document.execCommand('delete')
        return
      case 'Delete':
        if (editable(t)) document.execCommand('forwardDelete')
        return
      case 'PageDown':
      case 'PageUp':
        return scrollBy(0, (o.key === 'PageUp' ? -1 : 1) * innerHeight * 0.8)
      case 'ArrowDown':
      case 'ArrowUp': {
        const delta = o.key === 'ArrowUp' ? -1 : 1
        if (t.localName === 'select') {
          const i = Math.min(t.options.length - 1, Math.max(0, t.selectedIndex + delta))
          if (i === t.selectedIndex) return
          t.selectedIndex = i
          t.dispatchEvent(new Event('input', { bubbles: true }))
          t.dispatchEvent(new Event('change', { bubbles: true }))
        } else if (!editable(t)) scrollBy(0, delta * 40)
        return
      }
      case 'Home':
      case 'End':
        if (!editable(t)) scrollTo(0, o.key === 'Home' ? 0 : document.documentElement.scrollHeight)
        return
    }
    if (o.text && editable(t)) document.execCommand('insertText', false, o.text)
  }

  const secret = () => !!document.querySelector('input[type=password]')

  globalThis.__sq = { S, el, snapshot, find, read, point, area, aim, rect, focus, choose, checked, scroll, click, insert, press, secret }
})()
