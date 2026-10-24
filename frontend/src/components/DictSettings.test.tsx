import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { dict, type DictPluginDetailResp, type DictProviderResp } from '../api'
import { toast } from '../toast'
import DictSettings from './DictSettings'

vi.mock('../api', () => ({
  dict: {
    providers: vi.fn(),
    setPluginEnabled: vi.fn(),
    importPlugin: vi.fn(),
    deletePlugin: vi.fn(),
    pluginDetail: vi.fn(),
  },
}))
vi.mock('../toast', () => ({ toast: vi.fn() }))

const mocked = vi.mocked(dict)

function provider(overrides: Partial<DictProviderResp> = {}): DictProviderResp {
  return {
    id: 'youdao',
    label: '有道词典',
    short_label: '有道',
    description: '网易有道',
    version: '1.0.0',
    capabilities: ['lookup', 'suggest'],
    enabled: true,
    ...overrides,
  }
}

function detail(overrides: Partial<DictPluginDetailResp> = {}): DictPluginDetailResp {
  return {
    id: 'youdao',
    label: '有道词典',
    short_label: '有道',
    description: null,
    version: '1.0.0',
    enabled: true,
    capabilities: ['lookup', 'suggest'],
    hosts: ['dict.youdao.com'],
    lookup_request: { method: 'get', path: '/jsonapi', headers: [], form: [] },
    lookup: {
      fields: [
        { key: 'word', count: 1, names: [] },
        { key: 'definitions', count: 1, names: [] },
      ],
      strategies: [{ kind: 'pos_text', has_pos: false, has_condition: false, transforms: 1 }],
      vars: ['phonetic_us'],
      empty_is_parse: false,
    },
    suggest_request: null,
    suggest: { fields: ['word', 'summary'], take: 24, dedup: 'word_lowercase' },
    limits: {
      max_definitions: 20,
      max_examples: 5,
      max_body_kib: 1024,
      ceiling_definitions: 20,
      ceiling_examples: 5,
      ceiling_body_kib: 1024,
    },
    ...overrides,
  }
}

describe('DictSettings', () => {
  beforeEach(() => {
    vi.resetAllMocks()
  })

  afterEach(cleanup)

  it('只展示插件、开关、导入与删除', async () => {
    mocked.providers.mockResolvedValue([provider()])
    render(<DictSettings />)

    expect(await screen.findByText('有道词典')).toBeTruthy()
    expect(screen.getByRole('switch', { name: '有道词典' })).toBeTruthy()
    expect(screen.getByLabelText('导入插件')).toBeTruthy()
    expect(screen.getByRole('button', { name: '删除 有道词典' })).toBeTruthy()
    expect(screen.queryByText('内置')).toBeNull()
    expect(screen.queryByText('外部')).toBeNull()
    expect(screen.queryByText('已安装')).toBeNull()
    expect(screen.queryByText('重新加载')).toBeNull()
    expect(screen.queryByText('./plugins')).toBeNull()
  })

  it('点击源打开可视化详情并可返回', async () => {
    mocked.providers.mockResolvedValue([provider()])
    mocked.pluginDetail.mockResolvedValue(detail())
    render(<DictSettings />)

    fireEvent.click(await screen.findByRole('button', { name: '查看 有道词典 详情' }))

    expect(await screen.findByRole('heading', { name: '有道词典' })).toBeTruthy()
    expect(screen.getByText('注册内容')).toBeTruthy()
    expect(screen.getByText('释义策略')).toBeTruthy()
    expect(screen.getByText('模糊建议')).toBeTruthy()
    expect(screen.getAllByText('GET').length).toBeGreaterThanOrEqual(1)
    expect(mocked.pluginDetail).toHaveBeenCalledWith('youdao')

    fireEvent.click(screen.getByLabelText('返回词典源'))
    expect(await screen.findByRole('switch', { name: '有道词典' })).toBeTruthy()
  })

  it('详情加载失败可重试', async () => {
    mocked.providers.mockResolvedValue([provider()])
    mocked.pluginDetail.mockRejectedValueOnce(new Error('详情加载失败')).mockResolvedValueOnce(detail())
    render(<DictSettings />)

    fireEvent.click(await screen.findByRole('button', { name: '查看 有道词典 详情' }))
    expect((await screen.findByRole('alert')).textContent).toContain('详情加载失败')

    fireEvent.click(screen.getByRole('button', { name: '重试' }))
    expect(await screen.findByRole('heading', { name: '有道词典' })).toBeTruthy()
    expect(mocked.pluginDetail).toHaveBeenCalledTimes(2)
  })

  it('切换开关调用后端并更新状态', async () => {
    mocked.providers.mockResolvedValue([provider()])
    mocked.setPluginEnabled.mockResolvedValue(provider({ enabled: false }))
    render(<DictSettings />)

    const toggle = await screen.findByRole('switch', { name: '有道词典' })
    fireEvent.click(toggle)

    await waitFor(() => expect(mocked.setPluginEnabled).toHaveBeenCalledWith('youdao', false))
    await waitFor(() => expect(toggle.getAttribute('aria-checked')).toBe('false'))
  })

  it('切换失败时保留状态并提示', async () => {
    mocked.providers.mockResolvedValue([provider()])
    mocked.setPluginEnabled.mockRejectedValue(new Error('写入失败'))
    render(<DictSettings />)

    fireEvent.click(await screen.findByRole('switch', { name: '有道词典' }))

    await waitFor(() => expect(toast).toHaveBeenCalledWith('写入失败'))
    expect(screen.getByRole('switch', { name: '有道词典' }).getAttribute('aria-checked')).toBe('true')
  })

  it('导入插件后替换列表', async () => {
    mocked.providers.mockResolvedValue([])
    mocked.importPlugin.mockResolvedValue([provider({ id: 'custom', label: '自定义源' })])
    render(<DictSettings />)

    const input = await screen.findByLabelText('导入插件')
    const file = new File(['{}'], 'custom.json', { type: 'application/json' })
    fireEvent.change(input, { target: { files: [file] } })

    await waitFor(() => expect(mocked.importPlugin).toHaveBeenCalledWith(file))
    expect(await screen.findByText('自定义源')).toBeTruthy()
    expect(toast).toHaveBeenCalledWith('已导入')
  })

  it('确认后删除插件并替换列表', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true)
    mocked.providers.mockResolvedValue([provider()])
    mocked.deletePlugin.mockResolvedValue([])
    render(<DictSettings />)

    fireEvent.click(await screen.findByRole('button', { name: '删除 有道词典' }))

    await waitFor(() => expect(confirm).toHaveBeenCalled())
    await waitFor(() => expect(mocked.deletePlugin).toHaveBeenCalledWith('youdao'))
    expect(toast).toHaveBeenCalledWith('已删除')
    confirm.mockRestore()
  })
})
