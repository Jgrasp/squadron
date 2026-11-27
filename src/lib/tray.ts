import { defaultWindowIcon } from '@tauri-apps/api/app'
import { Menu } from '@tauri-apps/api/menu'
import { TrayIcon } from '@tauri-apps/api/tray'
import { getCurrentWindow } from '@tauri-apps/api/window'

export function showWindow() {
  const window = getCurrentWindow()
  return Promise.all([window.unminimize(), window.show()]).then(() => window.setFocus())
}

export async function tray(labels: { show: string; quit: string }, quit: () => void) {
  const menu = await Menu.new({
    items: [
      { id: 'show', text: labels.show, action: () => showWindow() },
      { id: 'quit', text: labels.quit, action: quit },
    ],
  })
  const icon = await TrayIcon.new({ icon: (await defaultWindowIcon()) ?? undefined, tooltip: 'Squadron', menu, showMenuOnLeftClick: false, action: (event) => event.type === 'Click' && event.button === 'Left' && showWindow() })
  return () => {
    icon.close()
    menu.close()
  }
}
