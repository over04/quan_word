import type { CSSProperties } from 'react'
import type { DictPluginDetailResp, DictProviderResp } from '../api'
import {
  BookIcon,
  ChevronLeftIcon,
  CodeIcon,
  FlowIcon,
  GlobeIcon,
  InboxIcon,
  LayersIcon,
  ListIcon,
  PaperIcon,
  QuoteIcon,
  SparkIcon,
  SpeakerIcon,
  TagIcon,
} from './Icons'

type Field = NonNullable<DictPluginDetailResp['lookup']>['fields'][number]
type Strategy = NonNullable<DictPluginDetailResp['lookup']>['strategies'][number]
type Request = NonNullable<DictPluginDetailResp['lookup_request']>
type Suggest = NonNullable<DictPluginDetailResp['suggest']>
type Limits = DictPluginDetailResp['limits']

const FIELD_META = {
  word: { label: '词头', Icon: PaperIcon },
  phonetic: { label: '音标', Icon: SpeakerIcon },
  audio: { label: '音频', Icon: SpeakerIcon },
  source: { label: '来源页', Icon: GlobeIcon },
  definitions: { label: '释义', Icon: LayersIcon },
  examples: { label: '例句', Icon: QuoteIcon },
} satisfies Record<Field['key'], { label: string; Icon: typeof PaperIcon }>

const STRATEGY_META = {
  pos_text: { label: '词性文本', Icon: TagIcon },
  plain_text: { label: '纯文本', Icon: PaperIcon },
  items: { label: '条目', Icon: ListIcon },
  groups: { label: '分组', Icon: LayersIcon },
} satisfies Record<Strategy['kind'], { label: string; Icon: typeof PaperIcon }>

type Props = {
  plugin: DictProviderResp
  detail: DictPluginDetailResp | null
  loading: boolean
  error: string
  onBack: () => void
  onRetry: () => void
}

/** 单个词典源的可视化档案：能力、请求、注册字段、释义策略、变量与上限。 */
export default function DictPluginDetail({ plugin, detail, loading, error, onBack, onRetry }: Props) {
  const capabilities = detail?.capabilities ?? plugin.capabilities
  const lookup = detail?.lookup ?? null
  const suggest = detail?.suggest ?? null
  const request = detail?.lookup_request ?? detail?.suggest_request ?? null
  const initial = (plugin.short_label || plugin.label).slice(0, 1)

  return (
    <section className="plugin-detail" aria-label={`${plugin.label} 详情`}>
      <header className="plugin-detail-head">
        <button type="button" className="plugin-back" onClick={onBack} aria-label="返回词典源">
          <ChevronLeftIcon className="h-4 w-4" />
        </button>
        <div className={`plugin-sigil ${plugin.enabled ? 'is-on' : ''}`} aria-hidden="true">
          <span className="plugin-sigil-ring" />
          <span className="plugin-sigil-ring plugin-sigil-ring--inner" />
          <span className="plugin-sigil-dot" />
          <span className="plugin-sigil-core">{initial}</span>
        </div>
        <div className="plugin-head-text">
          <div className="plugin-title-row">
            <h2 className="plugin-title">{plugin.label}</h2>
            <span className="plugin-version">v{plugin.version}</span>
          </div>
          <div className="plugin-meta">
            <span className={`plugin-status ${plugin.enabled ? 'is-on' : ''}`}>
              <span className="plugin-status-dot" aria-hidden="true" />
              {plugin.enabled ? '运行中' : '已停用'}
            </span>
            {detail && <span className="plugin-meta-item">{detail.hosts.length} 域</span>}
            {suggest && <span className="plugin-meta-item">建议 {suggest.take ?? 24}</span>}
          </div>
        </div>
        <div className="plugin-caps">
          <CapabilityMark label="查词" active={capabilities.includes('lookup')} Icon={BookIcon} />
          <CapabilityMark label="建议" active={capabilities.includes('suggest')} Icon={SparkIcon} />
        </div>
      </header>

      {loading && !detail ? (
        <div className="plugin-detail-skeleton" aria-label="正在加载插件详情">
          <span />
          <span />
          <span />
          <span />
        </div>
      ) : error ? (
        <div className="plugin-detail-error" role="alert">
          <span className="plugin-detail-error-mark" aria-hidden="true">!</span>
          <p>{error}</p>
          <button type="button" className="plugin-retry" onClick={onRetry}>重试</button>
        </div>
      ) : detail ? (
        <div className="plugin-detail-body">
          <div className="plugin-flow">
            <FlowNode label="请求" value={request ? (request.method === 'get' ? 'GET' : 'POST') : '—'} Icon={FlowIcon} index={0} />
            <FlowNode label="变量" value={lookup?.vars.length ?? 0} Icon={CodeIcon} index={1} />
            <FlowNode label="释义" value={lookup?.strategies.length ?? 0} Icon={LayersIcon} index={2} />
            <FlowNode label="输出" value={(lookup?.fields.length ?? 0) + (suggest?.fields.length ?? 0)} Icon={InboxIcon} index={3} />
          </div>

          <div className="plugin-detail-grid">
            <section className="plugin-card plugin-card--fields">
              <SectionLabel label="注册内容" count={lookup?.fields.length ?? 0} />
              <div className="plugin-fields">
                {(lookup?.fields ?? []).map((field, index) => (
                  <FieldTile key={field.key} field={field} index={index} />
                ))}
                {!lookup?.fields.length && <span className="plugin-empty-mark" aria-hidden="true" />}
              </div>

              {lookup && lookup.strategies.length > 0 && (
                <>
                  <SectionLabel label="释义策略" count={lookup.strategies.length} />
                  <div className="plugin-strategies">
                    {lookup.strategies.map((strategy, index) => (
                      <StrategyTile key={`${strategy.kind}-${index}`} strategy={strategy} index={index} />
                    ))}
                  </div>
                </>
              )}
            </section>

            <aside className="plugin-side plugin-side--main">
              {request && <RequestCard request={request} hosts={detail.hosts} />}
              {suggest && <SuggestCard suggest={suggest} />}
            </aside>
            <aside className="plugin-side plugin-side--aux">
              <VarsCard vars={lookup?.vars ?? []} />
              <LimitsCard limits={detail.limits} />
            </aside>
          </div>
        </div>
      ) : null}
    </section>
  )
}

function CapabilityMark({
  label,
  active,
  Icon,
}: {
  label: string
  active: boolean
  Icon: typeof BookIcon
}) {
  return (
    <span className={`plugin-cap ${active ? 'is-on' : ''}`}>
      <span className="plugin-cap-glyph">
        <Icon className="h-4 w-4" />
      </span>
      <span>{label}</span>
      <span className="plugin-cap-dot" aria-hidden="true" />
    </span>
  )
}

function FlowNode({
  label,
  value,
  Icon,
  index,
}: {
  label: string
  value: string | number
  Icon: typeof FlowIcon
  index: number
}) {
  return (
    <div className="plugin-flow-node" style={{ '--i': index } as CSSProperties}>
      <span className="plugin-flow-glyph">
        <Icon className="h-4 w-4" />
      </span>
      <span className="plugin-flow-label">{label}</span>
      <span className="plugin-flow-value">{value}</span>
    </div>
  )
}

function SectionLabel({ label, count }: { label: string; count?: number }) {
  return (
    <div className="plugin-section-label">
      <span>{label}</span>
      {typeof count === 'number' && <span className="plugin-section-count">{count}</span>}
    </div>
  )
}

function FieldTile({ field, index }: { field: Field; index: number }) {
  const meta = FIELD_META[field.key]
  const Icon = meta.Icon
  return (
    <div
      className="plugin-field"
      data-kind={field.key}
      title={field.names.length > 0 ? field.names.join(' · ') : undefined}
      style={{ '--i': index } as CSSProperties}
    >
      <span className="plugin-field-glyph">
        <Icon className="h-4 w-4" />
      </span>
      <span className="plugin-field-body">
        <span className="plugin-field-label">{meta.label}</span>
        {field.names.length > 0 && <span className="plugin-field-names">{shortNames(field.names)}</span>}
      </span>
      <span className="plugin-field-count">{field.count}</span>
      <span className="plugin-field-scan" aria-hidden="true" />
    </div>
  )
}

function StrategyTile({ strategy, index }: { strategy: Strategy; index: number }) {
  const meta = STRATEGY_META[strategy.kind]
  const Icon = meta.Icon
  return (
    <div className="plugin-strategy" data-kind={strategy.kind} style={{ '--i': index } as CSSProperties}>
      <span className="plugin-strategy-glyph">
        <Icon className="h-4 w-4" />
      </span>
      <span className="plugin-strategy-label">{meta.label}</span>
      <span className="plugin-strategy-marks">
        {strategy.has_pos && <span className="plugin-mark">POS</span>}
        {strategy.has_condition && <span className="plugin-mark">IF</span>}
        {strategy.transforms > 0 && <span className="plugin-mark">×{strategy.transforms}</span>}
      </span>
    </div>
  )
}

function RequestCard({ request, hosts }: { request: Request; hosts: string[] }) {
  return (
    <article className="plugin-card">
      <SectionLabel label="请求" />
      <div className="plugin-request-line">
        <span className="plugin-method">{request.method === 'get' ? 'GET' : 'POST'}</span>
        <span className="plugin-path" title={request.path}>{request.path}</span>
      </div>
      <div className="plugin-host-row">
        {hosts.map((host) => (
          <span className="plugin-host" key={host}>
            <GlobeIcon className="h-3.5 w-3.5" />
            <span className="plugin-host-name">{host}</span>
          </span>
        ))}
      </div>
      {(request.headers.length > 0 || request.form.length > 0) && (
        <div className="plugin-request-chips">
          {request.headers.length > 0 && (
            <span className="plugin-request-chip">
              <TagIcon className="h-3.5 w-3.5" />
              {shortNames(request.headers)}
            </span>
          )}
          {request.form.length > 0 && (
            <span className="plugin-request-chip">
              <InboxIcon className="h-3.5 w-3.5" />
              {shortNames(request.form)}
            </span>
          )}
        </div>
      )}
    </article>
  )
}

function SuggestCard({ suggest }: { suggest: Suggest }) {
  return (
    <article className="plugin-card plugin-card--suggest">
      <SectionLabel label="模糊建议" />
      <div className="plugin-suggest-head">
        <span className="plugin-suggest-take">×{suggest.take ?? 24}</span>
        {suggest.dedup && <span className="plugin-suggest-dedup">DEDUP</span>}
      </div>
      <div className="plugin-suggest-fields">
        {suggest.fields.map((field) => (
          <span className="plugin-field-chip" key={field}>{field}</span>
        ))}
      </div>
    </article>
  )
}

function VarsCard({ vars }: { vars: string[] }) {
  return (
    <article className="plugin-card">
      <SectionLabel label="变量" count={vars.length} />
      <div className="plugin-vars">
        {vars.length > 0 ? (
          vars.map((name) => <span className="plugin-var" key={name}>{name}</span>)
        ) : (
          <span className="plugin-empty-mark" aria-hidden="true" />
        )}
      </div>
    </article>
  )
}

function LimitsCard({ limits }: { limits: Limits }) {
  return (
    <article className="plugin-card">
      <SectionLabel label="上限" />
      <div className="plugin-limits">
        <LimitRing label="释义" value={limits.max_definitions} max={limits.ceiling_definitions} index={0} />
        <LimitRing label="例句" value={limits.max_examples} max={limits.ceiling_examples} index={1} />
        <LimitRing
          label="响应"
          value={limits.max_body_kib}
          max={limits.ceiling_body_kib}
          index={2}
          format={(value) => (value >= 1024 ? `${Math.round(value / 1024)}M` : `${value}K`)}
        />
      </div>
    </article>
  )
}

function LimitRing({
  label,
  value,
  max,
  index,
  format,
}: {
  label: string
  value: number
  max: number
  index: number
  format?: (value: number) => string
}) {
  const circumference = 2 * Math.PI * 17
  const ratio = max > 0 ? Math.min(1, value / max) : 0
  return (
    <div className="plugin-limit" style={{ '--i': index } as CSSProperties}>
      <svg viewBox="0 0 44 44" className="plugin-limit-ring" aria-hidden="true">
        <circle cx="22" cy="22" r="17" className="plugin-limit-track" />
        <circle
          cx="22"
          cy="22"
          r="17"
          className="plugin-limit-value"
          style={{
            '--circ': `${circumference}`,
            '--offset': `${circumference * (1 - ratio)}`,
          } as CSSProperties}
        />
      </svg>
      <span className="plugin-limit-number">{format ? format(value) : value}</span>
      <span className="plugin-limit-label">{label}</span>
    </div>
  )
}

function shortNames(names: string[]) {
  return names.length > 3 ? `${names.slice(0, 3).join(' · ')} +${names.length - 3}` : names.join(' · ')
}
