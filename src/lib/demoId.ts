export const DEMO = 'demo:'

export const isDemo = (value: unknown) => typeof value === 'string' && value.startsWith(DEMO)

const KEYS = ['id', 'agent', 'project', 'path', 'dir', 'workdir', 'projects', 'paths']

const names = (value: unknown) =>
  isDemo(value) || (Array.isArray(value) && value.some(isDemo)) || (!!value && typeof value === 'object' && ['id', 'project'].some((k) => isDemo((value as Record<string, unknown>)[k])))

export const touchesDemo = (args: unknown) => !!args && typeof args === 'object' && KEYS.some((k) => names((args as Record<string, unknown>)[k]))
