const SIGNS: [string, RegExp[]][] = [
  ['php', [/<\?php/, /\$\w+\s*(=|->|;|\))/, /->\w+\(/, /\bnamespace\s+[\w\\]+;/, /\b(public|private|protected)\s+(static\s+)?function\b/, /^use\s+[\w\\]+;/m]],
  ['typescript', [/\binterface\s+\w+\s*\{/, /\w\)?:\s*(string|number|boolean|void|unknown)\b/, /\btype\s+\w+\s*=/, /\bimport\s+type\b/]],
  ['javascript', [/\b(const|let|var)\s+\w+\s*=/, /=>/, /\bfunction\s*\w*\s*\(/, /\bimport\s+.+\s+from\s+['"]/, /\bconsole\.\w+\(/, /\);\s*$/m]],
  ['python', [/^\s*def\s+\w+\(.*\):\s*$/m, /^\s*(from\s+[\w.]+\s+)?import\s+\w+$/m, /^\s*(if|for|while|elif|else|class|try|except)\b.*:\s*$/m, /\bself\./]],
  ['rust', [/\bfn\s+\w+/, /\blet\s+mut\b/, /\bimpl\b/, /\w::\w+/, /\bpub\s+(fn|struct|enum|mod)\b/, /\b(Some|Ok|Err)\(/]],
  ['sql', [/\bselect\b[\s\S]+\bfrom\b/i, /\b(insert\s+into|update\s+\w+\s+set|delete\s+from|create\s+table)\b/i, /\bwhere\b/i, /\b(left\s+|inner\s+)?join\b/i]],
  ['html', [/<\/?(div|span|p|a|ul|li|section|body|head|html|button|input|form|template)\b/i, /<\/\w+>/]],
  ['twig', [/\{\{.*\}\}/, /\{%.*%\}/]],
  ['css', [/^[.#:]?[\w-][^{\n]*\{\s*$/m, /^\s*[\w-]+\s*:\s*[^;\n]+;\s*$/m]],
  ['yaml', [/^[\w-]+:\s*$/m, /^\s+[\w-]+:\s+\S/m, /^\s*-\s+\S/m]],
  ['bash', [/^\s*\$\s+\w/m, /\|\s*(grep|awk|sed|xargs|head|tail)\b/, /&&\s*\w/, /^\s*(cd|ls|git|npm|pnpm|sudo|docker|curl|rtk)\s/m]],
]

const EDITOR_MODES: Record<string, string> = { shellscript: 'bash', jsonc: 'json', typescriptreact: 'tsx', javascriptreact: 'jsx' }

export function editorLanguage(data: string) {
  try {
    const mode = String(JSON.parse(data).mode ?? '')
    return mode && mode !== 'plaintext' && mode !== 'markdown' ? (EDITOR_MODES[mode] ?? mode) : ''
  } catch {
    return ''
  }
}

export function guessLanguage(code: string) {
  const text = code.trim()
  if (/^[[{]/.test(text))
    try {
      JSON.parse(text)
      return 'json'
    } catch {}
  let best = ''
  let top = 1
  for (const [lang, signs] of SIGNS) {
    const score = signs.filter((sign) => sign.test(text)).length
    if (score > top) [best, top] = [lang, score]
  }
  if (best === 'javascript' && SIGNS[1][1].some((sign) => sign.test(text))) return 'typescript'
  return best
}

export function codeLanguage(text: string, editor: string) {
  if (text.trim().split('\n').length < 2 || text.includes('```')) return ''
  return editorLanguage(editor) || guessLanguage(text)
}

export function fenceAt(value: string, start: number, end: number, code: string, lang: string) {
  const before = value.slice(0, start)
  if ((before.match(/^```/gm)?.length ?? 0) % 2) return ''
  const open = before && !before.endsWith('\n') ? '\n' : ''
  const close = value.slice(end).startsWith('\n') ? '' : '\n'
  return `${open}\`\`\`${lang}\n${code.replace(/\s+$/, '')}\n\`\`\`${close}`
}

export function fences(text: string) {
  const parts: { code: boolean; text: string }[] = []
  let at = 0
  for (const match of text.matchAll(/^```[^\n]*\n[\s\S]*?\n```[ \t]*$/gm)) {
    const prose = text.slice(at, match.index).replace(/^\n+|\n+$/g, '')
    if (prose) parts.push({ code: false, text: prose })
    parts.push({ code: true, text: match[0] })
    at = match.index + match[0].length
  }
  const rest = text.slice(at).replace(/^\n+|\n+$/g, '')
  if (rest || !parts.length) parts.push({ code: false, text: at ? rest : text })
  return parts
}
