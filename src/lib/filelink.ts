export function projectFile(href: string) {
  if (/^([a-z][a-z0-9+.-]*:|[/#?])/i.test(href)) return ''
  let path = href.replace(/^\.\//, '')
  try {
    path = decodeURI(path)
  } catch {}
  return /^[^?#]*[^/]\.[a-z0-9]{1,8}$/i.test(path) && !path.split('/').includes('..') ? path : ''
}

export function fenced(text: string, lang = '') {
  const fence = '`'.repeat(Math.max(2, ...(text.match(/`+/g) ?? []).map((run) => run.length)) + 1)
  return `${fence}${lang}\n${text}\n${fence}`
}

export const windowLabel = (project: string, path: string) =>
  `file-${([...`${project}\n${path}`].reduce((h, c) => (Math.imul(31, h) + c.charCodeAt(0)) | 0, 0) >>> 0).toString(36)}`

export function localFile(href: string, windows = navigator.userAgent.includes('Windows')) {
  let path = href.replace(/^file:\/\//, '')
  try {
    path = decodeURI(path)
  } catch {}
  if (windows) path = path.replaceAll('\\', '/').replace(/^\/([a-z])(?=\/)/i, '$1:').replace(/^\/([a-z]:\/)/i, '$1')
  return (path.startsWith('/') || (windows && /^[a-z]:\//i.test(path))) && /\/[^/]+\.[a-z0-9]{1,8}$/i.test(path) ? path : ''
}

export function within(path: string, project: string) {
  const root = project.replaceAll('\\', '/')
  return !/^([a-z]:)?\//i.test(path) ? path : root && path.startsWith(`${root}/`) ? path.slice(root.length + 1) : ''
}
