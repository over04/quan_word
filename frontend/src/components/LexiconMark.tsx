interface Props {
  className?: string
}

/** 查词首页的图标字标：摊开的词典 + 放大镜（镜片里是 Aa）。
 *  纯线稿，主色 currentColor，放大镜用陶土色；不依赖图片资源。 */
export default function LexiconMark({ className }: Props) {
  return (
    <svg
      viewBox="0 0 96 96"
      className={className}
      fill="none"
      aria-hidden="true"
      focusable="false"
    >
      <circle cx="48" cy="48" r="45" stroke="currentColor" strokeOpacity="0.16" />
      <circle
        className="lex-mark-ring"
        cx="48"
        cy="48"
        r="37"
        stroke="currentColor"
        strokeOpacity="0.34"
        strokeDasharray="1.5 5"
      />
      <path
        d="M25 35.5c7.8-3.9 15.2-3.9 23 0v28c-7.8-3.9-15.2-3.9-23 0v-28Z"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinejoin="round"
      />
      <path
        d="M71 35.5c-7.8-3.9-15.2-3.9-23 0v28c7.8-3.9 15.2-3.9 23 0v-28Z"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinejoin="round"
      />
      <path d="M48 35.5v28" stroke="currentColor" strokeOpacity="0.4" strokeWidth="1.2" />
      <path d="M31 43c4.5-2 9-2 13 0M31 49c4.5-2 9-2 13 0" stroke="currentColor" strokeOpacity="0.3" strokeWidth="1.1" strokeLinecap="round" />
      <path d="M56 43c3-1.3 6-1.3 9 0" stroke="currentColor" strokeOpacity="0.3" strokeWidth="1.1" strokeLinecap="round" />
      <circle cx="60" cy="34" r="10.5" fill="var(--color-ivory)" stroke="var(--color-clay)" strokeWidth="2" />
      <path d="m67.5 41.5 7.2 7.2" stroke="var(--color-clay)" strokeWidth="2.4" strokeLinecap="round" />
      <text
        x="60"
        y="37.5"
        textAnchor="middle"
        fill="var(--color-clay)"
        fontFamily="'DM Serif Display', 'Songti SC', serif"
        fontSize="9.5"
      >
        Aa
      </text>
      <circle cx="48" cy="79" r="1.5" fill="var(--color-clay)" />
    </svg>
  )
}
