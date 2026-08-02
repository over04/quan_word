/** 柔和波浪分隔线（style.md：以 SVG 曲线代替直线分隔） */
export default function WavyDivider() {
  return (
    <svg
      className="w-full h-3 text-clay/35"
      viewBox="0 0 600 12"
      fill="none"
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      <path
        d="M0 6 C 50 12, 100 0, 150 6 S 250 12, 300 6 S 400 0, 450 6 S 550 12, 600 6"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  )
}
