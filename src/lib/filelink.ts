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

export const within = (path: string, project: string) =>
  !path.startsWith('/') ? path : project && path.startsWith(`${project}/`) ? path.slice(project.length + 1) : ''
