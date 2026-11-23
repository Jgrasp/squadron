import { describe, expect, it } from 'vitest'
import { isUnread, markSeen, settle } from './unread.svelte'
import type { Remark } from './pulls.svelte'

const remark = (id: string, author: string): Remark => ({ id, author, avatar: '', body: id, path: null, line: null, removed: false, reply_to: null, at: '', kind: 'general' })

describe('unread remarks', () => {
  it('takes the first look as read, then flags what others add', () => {
    settle('p#1', [remark('a', 'marie')])
    expect(isUnread('p#1', remark('a', 'marie'), 'quentin')).toBe(false)
    settle('p#1', [remark('a', 'marie'), remark('b', 'thomas')])
    expect(isUnread('p#1', remark('b', 'thomas'), 'quentin')).toBe(true)
    expect(isUnread('p#1', remark('c', 'quentin'), 'quentin')).toBe(false)
    markSeen('p#1', ['b'])
    expect(isUnread('p#1', remark('b', 'thomas'), 'quentin')).toBe(false)
  })
})
