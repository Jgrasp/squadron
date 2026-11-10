import { invoke } from '@tauri-apps/api/core'

export type Listed = { name: string; description: string; command: string }

export const mcp = $state({ onDemand: [] as Listed[], everywhere: [] as Listed[], loaded: false })

export async function loadServers() {
  const found = await invoke<{ on_demand: Listed[]; everywhere: Listed[] }>('mcp_servers')
  mcp.onDemand = found.on_demand
  mcp.everywhere = found.everywhere
  mcp.loaded = true
  return found
}
