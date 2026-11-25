export const DEMO = 'demo:'

export const isDemo = (value: unknown) => typeof value === 'string' && value.startsWith(DEMO)

export const touchesDemo = (args: unknown) => JSON.stringify(args ?? {}).includes(`"${DEMO}`)
