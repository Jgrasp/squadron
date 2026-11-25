import { core } from '@tauri-apps/api'

type Route = (cmd: string, args: Record<string, unknown>) => Promise<unknown> | undefined

export const ipc: { route: Route | null } = { route: null }

export const { Channel, PluginListener, Resource, SERIALIZE_TO_IPC_FN, addPluginListener, checkPermissions, convertFileSrc, isTauri, requestPermissions, transformCallback } = core

export function invoke<T>(cmd: string, args: Record<string, unknown> = {}, options?: Parameters<typeof core.invoke>[2]): Promise<T> {
  return (ipc.route?.(cmd, args) as Promise<T> | undefined) ?? core.invoke<T>(cmd, args, options)
}
