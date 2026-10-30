export function projectFile(href: string) {
  if (/^([a-z][a-z0-9+.-]*:|[/#?])/i.test(href)) return ''
  let path = href.replace(/^\.\//, '')
  try {
    path = decodeURI(path)
  } catch {}
  return /^[^?#]*[^/]\.[a-z0-9]{1,8}$/i.test(path) && !path.split('/').includes('..') ? path : ''
}
