import { afterEach, expect, it } from 'vitest'
import { loadReaderPageSize } from './readerPageSize'

afterEach(() => localStorage.removeItem('qw_page_size'))

it.each([['200', 100], ['100', 100], ['20', 20], ['invalid', 30]])(
  '已保存的页容量 %s 读取为 %d', (stored, expected) => {
    localStorage.setItem('qw_page_size', stored)
    expect(loadReaderPageSize()).toBe(expected)
  },
)
