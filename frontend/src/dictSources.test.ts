import { describe, expect, it } from 'vitest'
import { buildSourceRows } from './dictSources'
import type { DictEntry, DictProviderResp } from './api'

function provider(id: string, label: string, short = '', enabled = true): DictProviderResp {
  return {
    id,
    label,
    short_label: short,
    description: null,
    version: '1.0.0',
    capabilities: ['lookup'],
    enabled,
  }
}

function entry(providerId: string, label: string): DictEntry {
  return {
    provider: providerId,
    label,
    word: 'abandon',
    phonetic: null,
    definitions: [],
    examples: [],
    audio_url: null,
    source_url: null,
  }
}

describe('buildSourceRows', () => {
  it('keeps enabled plugin order and hides disabled plugins', () => {
    const rows = buildSourceRows(
      [provider('youdao', '有道词典', '有道'), provider('baidu', '百度翻译', '百度', false)],
      [],
    )
    expect(rows.map((row) => row.id)).toEqual(['youdao'])
    expect(rows[0].short).toBe('有道')
    expect(rows[0].available).toBe(false)
  })

  it('marks providers that returned an entry as available', () => {
    const rows = buildSourceRows(
      [provider('youdao', '有道词典'), provider('bing', '必应词典')],
      [entry('bing', '必应词典')],
    )
    expect(rows.find((row) => row.id === 'bing')?.available).toBe(true)
    expect(rows.find((row) => row.id === 'youdao')?.available).toBe(false)
  })

  it('appends entries whose plugin is not in the list yet', () => {
    const rows = buildSourceRows([], [entry('custom', '自定义源')])
    expect(rows).toEqual([
      { id: 'custom', label: '自定义源', short: '自定义源', available: true },
    ])
  })

  it('falls back to label when short_label is empty', () => {
    const rows = buildSourceRows([provider('youdao', '有道词典', '')], [])
    expect(rows[0].short).toBe('有道词典')
  })
})
