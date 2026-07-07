interface LogoProps {
  size?: number;
  className?: string;
}

export function Logo({ size = 20, className }: LogoProps) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 200 200"
      width={size}
      height={size}
      className={className}
      fill="none"
    >
      <defs>
        <linearGradient id="logo-g1" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#6366F1" />
          <stop offset="100%" stopColor="#4338CA" />
        </linearGradient>
        <linearGradient id="logo-g2" x1="100%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#10B981" />
          <stop offset="100%" stopColor="#047857" />
        </linearGradient>
        <linearGradient id="logo-g3" x1="0%" y1="100%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#A855F7" />
          <stop offset="100%" stopColor="#6D28D9" />
        </linearGradient>
        <linearGradient id="logo-g4" x1="100%" y1="100%" x2="0%" y2="0%">
          <stop offset="0%" stopColor="#F43F5E" />
          <stop offset="100%" stopColor="#BE123C" />
        </linearGradient>
      </defs>
      <g transform="translate(100,100) scale(1.6)">
        <polygon points="0,-50 35,-15 -35,-15" fill="url(#logo-g1)" opacity="0.95" />
        <polygon points="0,-50 0,0 35,-15" fill="#FFF" opacity="0.15" />
        <polygon points="0,0 35,-15 50,30" fill="url(#logo-g2)" opacity="0.9" />
        <polygon points="0,0 50,30 -50,30" fill="url(#logo-g3)" opacity="0.85" />
        <polygon points="0,50 50,30 -50,30" fill="url(#logo-g3)" opacity="0.95" />
        <polygon points="0,0 -50,30 -35,-15" fill="url(#logo-g4)" opacity="0.9" />
        <line x1="0" y1="-50" x2="0" y2="50" stroke="#FFF" strokeOpacity="0.3" strokeWidth="1.5" />
        <line x1="-50" y1="30" x2="50" y2="30" stroke="#FFF" strokeOpacity="0.2" strokeWidth="1" />
        <circle cx="0" cy="0" r="4" fill="#FFF" />
      </g>
    </svg>
  );
}
