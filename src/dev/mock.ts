import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'

type Project = { name: string; path: string }

const projects: Project[] = [
  { name: 'BMC', path: '/home/quentin/agence/bmc' },
  { name: 'Fileas', path: '/home/quentin/agence/fileas' },
  { name: 'Lumen', path: '/home/quentin/agence/lumen-absent' },
]

if (location.search.includes('many'))
  for (const name of ['Mercure', 'Atlas', 'Optika', 'Vinéa', 'Kalia', 'Sorbet', 'Néréide', 'Pixel Rando'])
    projects.push({ name, path: `/home/quentin/agence/${name.toLowerCase()}` })

const folder = (name: string, path: string) => ({
  name,
  path,
  exists: !path.includes('absent'),
  git: !path.includes('fileas'),
  branch: path.includes('fileas') ? null : 'feat/T15-segmentation',
})

const views = () => projects.map((p) => folder(p.name, p.path))

mockWindows('main')
mockIPC(
  (cmd, args) => {
    const a = args as Record<string, string>
    switch (cmd) {
      case 'list_projects':
        return views()
      case 'inspect_folder':
        return {
          ...folder(a.path.split('/').pop() ?? '', a.path),
          taken_by: projects.find((p) => p.path === a.path)?.name ?? null,
        }
      case 'add_project':
        projects.push({ name: a.name, path: a.path })
        return views()
      case 'remove_project':
        projects.splice(
          projects.findIndex((p) => p.path === a.path),
          1,
        )
        return views()
      case 'move_project':
        projects.find((p) => p.path === a.path)!.path = a.to
        return views()
      case 'plugin:dialog|open':
        return '/home/quentin/agence/mercure'
      case 'plugin:dialog|ask':
        return true
    }
  },
  { shouldMockEvents: true },
)
