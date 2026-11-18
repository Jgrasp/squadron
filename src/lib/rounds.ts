import type { Comment } from './stream'

export function byHead(list: Comment[], head: string) {
  const heads = [...new Set(list.map((c) => c.head || head))]
  return (heads.length ? heads : [head]).map((at) => ({ at, items: list.filter((c) => (c.head || head) === at) }))
}
