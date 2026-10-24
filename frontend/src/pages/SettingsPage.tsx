import { useState } from 'react'
import DataSettings from '../components/DataSettings'
import DictSettings from '../components/DictSettings'

const SECTIONS = [
  { id: 'dict', label: '词典源' },
  { id: 'data', label: '数据' },
] as const

type SectionId = (typeof SECTIONS)[number]['id']

/** 设置壳层：分区导航 + 内容槽，新增设置只加一个 section。 */
export default function SettingsPage() {
  const [active, setActive] = useState<SectionId>('dict')

  return (
    <section
      className="settings-page max-w-7xl mx-auto px-4 md:px-8 animate-page-in overflow-hidden"
      id="settings"
    >
      <div className="settings-orbit" aria-hidden="true">
        <span className="settings-orbit-ticks" />
        <span className="settings-orbit-ring settings-orbit-ring--outer" />
        <span className="settings-orbit-ring settings-orbit-ring--mid" />
        <span className="settings-orbit-ring settings-orbit-ring--inner" />
        <span className="settings-orbit-axis settings-orbit-axis--x" />
        <span className="settings-orbit-axis settings-orbit-axis--y" />
        <span className="settings-orbit-sweep" />
        <span className="settings-orbit-dot settings-orbit-dot--a" />
        <span className="settings-orbit-dot settings-orbit-dot--b" />
        <span className="settings-orbit-core" />
      </div>
      <div className="relative z-10 md:grid md:grid-cols-[10rem_minmax(0,1fr)] md:gap-8">
        <aside className="settings-aside">
          <h1 className="settings-title font-serif text-3xl text-charcoal leading-tight">
            <span>设</span>
            <span>置</span>
            <span className="settings-title-dot" aria-hidden="true" />
          </h1>
          <nav
            aria-label="设置分区"
            className="settings-nav relative mt-4 flex gap-1 overflow-x-auto md:flex-col md:overflow-visible"
          >
            <span className="settings-nav-rail" aria-hidden="true" />
            {SECTIONS.map((section) => {
              const current = active === section.id
              return (
                <button
                  key={section.id}
                  type="button"
                  aria-current={current ? 'page' : undefined}
                  onClick={() => setActive(section.id)}
                  className={`settings-nav-item min-h-11 shrink-0 rounded-full px-4 text-sm font-medium text-left ${
                    current
                      ? 'is-active bg-charcoal text-ivory'
                      : 'text-charcoal/55 hover:bg-white hover:text-charcoal'
                  }`}
                >
                  {section.label}
                </button>
              )
            })}
          </nav>
        </aside>
        <div className="settings-content mt-5 min-w-0 md:mt-0">
          <div className="settings-stage">
            <span className="settings-stage-rule" aria-hidden="true" />
            <div key={active} className="settings-panel">
              {active === 'dict' ? <DictSettings /> : <DataSettings />}
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
