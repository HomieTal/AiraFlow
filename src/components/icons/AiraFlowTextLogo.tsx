/** Aira Flow wordmark: mic glyph + "Aira Flow" text, both theme-blue. */
const AiraFlowTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  const VB_W = 232;
  const VB_H = 56;
  const w = width ?? 120;
  const h = height ?? (w * VB_H) / VB_W;
  return (
    <svg
      width={w}
      height={h}
      viewBox={`0 0 ${VB_W} ${VB_H}`}
      className={className}
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      role="img"
      aria-label="Aira Flow"
    >
      <g fill="var(--color-logo-primary)" stroke="var(--color-logo-primary)">
        {/* mic capsule */}
        <rect x="6" y="6" width="16" height="30" rx="8" />
        {/* holder arc */}
        <path
          d="M 2 38 A 12 12 0 0 0 26 38"
          strokeWidth="4.5"
          strokeLinecap="round"
          fill="none"
        />
        {/* stem */}
        <line
          x1="14"
          y1="43"
          x2="14"
          y2="52"
          strokeWidth="4.5"
          strokeLinecap="round"
        />
      </g>
      <text
        x="36"
        y="40"
        fill="var(--color-logo-primary)"
        style={{
          fontFamily: "var(--font-display, system-ui, sans-serif)",
          fontWeight: 700,
          letterSpacing: "-0.5px",
        }}
        fontSize="34"
        textLength="190"
        lengthAdjust="spacingAndGlyphs"
      >
        Aira Flow
      </text>
    </svg>
  );
};

export default AiraFlowTextLogo;
