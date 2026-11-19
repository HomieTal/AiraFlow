/** Aira Flow brand mark: mic glyph, theme-aware (`mascot-line` color). */
const AiraFlowMascot = ({ className = "" }: { className?: string }) => {
  return (
    <svg
      viewBox="0 0 100 100"
      className={className}
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <g
        fill="var(--color-mascot-line)"
        stroke="var(--color-mascot-line)"
      >
        {/* mic capsule */}
        <rect x="42" y="8" width="16" height="34" rx="8" />
        {/* holder arc */}
        <path
          d="M 26 46 A 24 24 0 0 0 74 46"
          strokeWidth="9"
          strokeLinecap="round"
          fill="none"
        />
        {/* stem */}
        <line
          x1="50"
          y1="62"
          x2="50"
          y2="88"
          strokeWidth="9"
          strokeLinecap="round"
        />
      </g>
    </svg>
  );
};

export default AiraFlowMascot;
