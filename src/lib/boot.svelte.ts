export const boot = $state({ settings: false, projects: false, agents: -1, pulls: false })

let finish = () => {}
export const finished = new Promise<void>((resolve) => (finish = resolve))
export const done = () => finish()
