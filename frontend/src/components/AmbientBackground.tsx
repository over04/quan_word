import { createPortal } from 'react-dom'

/** 页面底层的有机光斑 + 漂浮字形。
 *
 * 定位：整层通过 portal 挂到 <body> 下，且 .ambient-stage 在 CSS 里写死
 * position: fixed; inset: 0; z-index: 0; pointer-events: none。
 * 原因：position: fixed 会被任何带 transform / filter / backdrop-filter /
 * perspective / contain: paint / will-change 的祖先"捕获"成相对该祖先定位，
 * 一旦祖先（页面容器、动画包裹层）带上这些属性，背景就会跟着内容一起滚。
 * 挂到 body 下就没有任何祖先能捕获它，滚动时纹丝不动。
 *
 * 性能取舍（移动端同样保留视觉丰富度）：
 * - 三个色斑的 filter: blur() 是静态的：元素被提升为合成层后只光栅化一次，
 *   之后每帧只做合成，blur 不会逐帧重算；色斑本身不参与动画。
 * - 轮廓环不模糊，只做超慢旋转；字形层只做 transform/opacity。
 * - 容器 contain: layout paint style，绘制与布局与页面其它部分隔离。
 * 移动端不再隐藏，只缩小尺寸/降低模糊半径，观感保留、开销可控。
 */

interface Glyph {
  ch: string
  left: string
  top: string
  size: string
  rot: string
  dx: string
  dy: string
  dur: string
  delay: string
  opacity: string
  color: string
}

const GLYPHS: Glyph[] = [
  { ch: 'A', left: '3%', top: '20%', size: '2.6rem', rot: '-9deg', dx: '16px', dy: '-26px', dur: '24s', delay: '0s', opacity: '0.55', color: 'rgb(197 143 109 / 0.32)' },
  { ch: 'b', left: '89%', top: '15%', size: '2rem', rot: '8deg', dx: '-14px', dy: '-20px', dur: '29s', delay: '-7s', opacity: '0.6', color: 'rgb(47 42 37 / 0.16)' },
  { ch: 'c', left: '6%', top: '58%', size: '2.2rem', rot: '6deg', dx: '20px', dy: '-14px', dur: '26s', delay: '-13s', opacity: '0.5', color: 'rgb(47 42 37 / 0.13)' },
  { ch: 'd', left: '86%', top: '64%', size: '2.8rem', rot: '-6deg', dx: '-18px', dy: '-24px', dur: '31s', delay: '-4s', opacity: '0.55', color: 'rgb(197 143 109 / 0.28)' },
  { ch: 'e', left: '4%', top: '84%', size: '1.8rem', rot: '9deg', dx: '12px', dy: '-18px', dur: '23s', delay: '-16s', opacity: '0.5', color: 'rgb(47 42 37 / 0.12)' },
  { ch: 'f', left: '90%', top: '88%', size: '2.1rem', rot: '-8deg', dx: '-16px', dy: '-22px', dur: '27s', delay: '-9s', opacity: '0.55', color: 'rgb(197 143 109 / 0.3)' },
]

function AmbientLayer() {
  return (
    <div aria-hidden="true" className="ambient-stage">
      <div className="ambient-orb ambient-orb-a bg-sage/50" />
      <div className="ambient-orb ambient-orb-b bg-clay/25" />
      <div className="ambient-orb ambient-orb-c bg-sand/70" />
      <div className="ambient-halo ambient-halo-a" />
      <div className="ambient-halo ambient-halo-b" />
      {GLYPHS.map((glyph) => (
        <span
          key={glyph.ch}
          className="ambient-glyph"
          style={
            {
              left: glyph.left,
              top: glyph.top,
              fontSize: glyph.size,
              color: glyph.color,
              '--glyph-rot': glyph.rot,
              '--glyph-dx': glyph.dx,
              '--glyph-dy': glyph.dy,
              '--glyph-dur': glyph.dur,
              '--glyph-delay': glyph.delay,
              '--glyph-op': glyph.opacity,
            } as React.CSSProperties
          }
        >
          {glyph.ch}
        </span>
      ))}
    </div>
  )
}

interface Props {
  /**
   * 就地渲染（不进 portal）。全屏覆盖层（词典详情 z-50）必须用这个：
   * portal 到 body 的节点在根层叠上下文里 z-index 0，会被不透明的覆盖层整片盖住。
   * 就地渲染时它是覆盖层自己的子节点，画在覆盖层底色之上、内容之下。
   */
  inline?: boolean
}

export default function AmbientBackground({ inline = false }: Props) {
  // 页面壳层用 portal 直接挂到 body：任何祖先的 transform/filter/contain 都影响不到它
  if (inline) return <AmbientLayer />
  return createPortal(<AmbientLayer />, document.body)
}
