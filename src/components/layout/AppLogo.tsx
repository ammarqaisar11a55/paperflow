/** The PaperFlow mark, drawn inline so it follows the theme's accent. */
export function AppLogo({ size = 24 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 1024 1024" aria-hidden="true" className="shrink-0">
      <rect x="64" y="64" width="896" height="896" rx="200" fill="var(--accent)" />
      <path
        d="M332 232 H588 L716 360 V752 a40 40 0 0 1 -40 40 H332 a40 40 0 0 1 -40 -40 V272 a40 40 0 0 1 40 -40 Z"
        fill="#fff"
      />
      <path d="M588 232 V320 a40 40 0 0 0 40 40 H716 Z" fill="#D9DAFB" />
      <path
        d="M356 600 C 430 520, 500 680, 576 600 S 660 540, 668 560"
        fill="none"
        stroke="#5B5CE2"
        strokeWidth="56"
        strokeLinecap="round"
      />
    </svg>
  );
}
