import { execFile } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { promisify } from 'node:util'
import { describe, expect, it } from 'vitest'

type Case = [text: string, wanted: string[]]

const quick = ['haiku · low', 'haiku · medium']
const usual = ['sonnet · medium', 'sonnet · high']
const deep = ['opus · high', 'opus · xhigh']

const cases: Case[] = [
  ['Corrige la faute : « recevoire » dans src/lib/i18n/fr/common.ts', quick],
  ['Dans src/lib/chime.ts mets VOLUME à 0.4', quick],
  ['Traduis le libellé « Archive » en français dans src/lib/i18n/fr/kanban.ts', quick],
  ["C'est quoi la différence entre git rebase et git merge ?", quick],
  ['Comment on écrit une regex qui matche un email ?', quick],
  ["C'est quoi CORS en deux phrases ?", quick],
  ['Set the default port to 8080 in config/server.toml', quick],
  ['What does a 502 status code mean?', quick],

  ['Bug : On voit pas le auto ici', usual],
  ["Je suis déjà à 8% alors que ça fait que 10min que j'ai commencé. Pourquoi ça tabasse autant mon abonnement aujourd'hui ????", usual],
  ['Renomme le bouton Envoyer en Soumettre', [...quick, ...usual]],
  ['ça crash au démarrage', usual],
  ["Pourquoi l'app est lente quand j'ouvre un gros projet ?", usual],
  ['Ajoute un test pour la fonction settle dans autoLaunch', usual],
  ['le push marche plus', usual],
  ['Les onglets se ferment tout seuls quand je change de projet', usual],
  ['La CI met 25 min, tu peux regarder ?', [...usual, 'opus · high']],
  ['Relis la PR 98 et dis-moi si c’est bon', [...usual, ...deep]],
  ['Le badge rouge est moche', usual],
  ['Quelle version de Tauri on utilise ?', ['sonnet · low', ...usual]],
  ['Trie les agents par date de dernière activité dans la sidebar', usual],
  ['The login form accepts empty passwords', usual],
  ['Add a copy button to code blocks', usual],

  ['Refonds le système de permissions pour supporter des rôles par projet', deep],
  ["Je veux pouvoir brancher des thèmes tiers dans l'app, propose une archi", deep],
  ["La RAM monte à 4 Go après une journée d'utilisation", deep],
  ['Les messages se perdent parfois quand deux agents écrivent en même temps', deep],
  ['Migre la base de données de JSON vers SQLite', deep],
  ['Ajoute le support de GitLab en plus de GitHub', deep],
  ['Memory usage keeps growing until the app gets killed', deep],
]

const fresh: Case[] = [
  ['Le scroll saute en haut quand un nouveau message arrive', usual],
  ['Mets à jour vite en version 8.4', ['sonnet · low', ...usual]],
  ['Explain what a mutex is', quick],
  ['Supprime la clé `beta` de src/lib/i18n/en/settings.ts', quick],
  ['Les notifications arrivent en double, mais pas toujours', deep],
  ['Write the README section on installing on Windows', usual],
  ['Passe tout le front de Svelte à React', ['opus · xhigh', 'opus · high']],
  ["Je comprends pas pourquoi le menu ne s'ouvre pas sur macOS", usual],
]

const LABELS: Record<string, string[]> = { h: quick, hs: [...quick, ...usual], s: usual, so: [...usual, ...deep], o: deep }
const local = new URL('./auto.cases.local.json', import.meta.url)
type Real = { text: string; label: string; holdout?: boolean }
const real: Real[] = existsSync(local) ? JSON.parse(readFileSync(local, 'utf8')) : []
const realCases = (holdout: boolean): Case[] => real.filter((c) => !!c.holdout === holdout).map((c) => [c.text, LABELS[c.label]])

const RUNS = Number(process.env.AUTO_EVAL_RUNS ?? 3)
const prompt = readFileSync(new URL('../src/lib/autoPrompt.md', import.meta.url), 'utf8').trim()
const rust = readFileSync(new URL('../src-tauri/src/lib.rs', import.meta.url), 'utf8')
const format = JSON.parse(`"${/const LAUNCH_FORMAT: &str = "(.*)";/.exec(rust)![1]}"`)
const [judge, judgeEffort] = /const LAUNCH_JUDGE: \(&str, &str\) = \("(\w+)", "(\w+)"\);/.exec(rust)!.slice(1)
const run = promisify(execFile)

async function pick(text: string) {
  const args = ['-p', '--model', judge, '--effort', judgeEffort, '--output-format', 'json', '--tools', '', '--strict-mcp-config', '--setting-sources', 'project', '--no-session-persistence', '--system-prompt', `${prompt}\n\n${format}`, '--', text]
  const { stdout } = await run('claude', args, { cwd: tmpdir(), timeout: 20_000 })
  try {
    const result: string = JSON.parse(stdout).result
    const { model, effort } = JSON.parse(result.slice(result.indexOf('{'), result.lastIndexOf('}') + 1))
    return `${model} · ${effort}`
  } catch {
    return `no pick: ${stdout.slice(0, 300)}`
  }
}

async function check(text: string, wanted: string[]) {
  const picks = await Promise.all(Array.from({ length: RUNS }, () => pick(text)))
  expect(picks.filter((p) => !wanted.includes(p)), `wanted ${wanted.join(' or ')}`).toEqual([])
}

describe.runIf(process.env.AUTO_EVAL)('auto launch picks', () => {
  it.concurrent.each(cases)('%s', check, 60_000)
})

describe.runIf(process.env.AUTO_EVAL)('auto launch picks on messages the prompt was not tuned on', () => {
  it.concurrent.each(fresh)('%s', check, 60_000)
})

describe.runIf(process.env.AUTO_EVAL && real.length)('auto launch picks on real first messages', () => {
  it.concurrent.each(realCases(false))('%s', check, 60_000)
})

describe.runIf(process.env.AUTO_EVAL && real.length)('auto launch picks on real first messages the prompt was not tuned on', () => {
  it.concurrent.each(realCases(true))('%s', check, 60_000)
})
