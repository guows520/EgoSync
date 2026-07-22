const logoUrl = new URL('../../assets/egosync-logo.svg', import.meta.url).href;

interface LogoProps {
  size?: number;
  className?: string;
}

/** EgoSync 的核心标志：围绕同一中心彼此衔接的双向循环。 */
export function Logo({ size = 20, className }: LogoProps) {
  return (
    <img
      src={logoUrl}
      width={size}
      height={size}
      className={className}
      alt=""
      aria-hidden="true"
      draggable={false}
    />
  );
}

