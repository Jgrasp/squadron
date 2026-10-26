import type { Catalog } from '../index.svelte'
import agent from './agent'
import board from './board'
import common from './common'
import conversation from './conversation'
import format from './format'
import review from './review'
import settings from './settings'
import shell from './shell'

export default { common, format, settings, board, review, agent, conversation, shell } satisfies Catalog
