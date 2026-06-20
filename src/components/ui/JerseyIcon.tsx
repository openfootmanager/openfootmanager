import type { KitPattern } from "../../store/types";

interface JerseyIconProps {
  primaryColor: string;
  secondaryColor: string;
  pattern: KitPattern;
  number?: number | null;
  size?: "sm" | "md" | "lg";
  className?: string;
}

const SIZE_MAP = { sm: 32, md: 48, lg: 72 };

export default function JerseyIcon({
  primaryColor,
  secondaryColor,
  pattern,
  number,
  size = "md",
  className,
}: JerseyIconProps) {
  const px = SIZE_MAP[size];
  const id = `jersey-${pattern}-${primaryColor.replace("#", "")}-${secondaryColor.replace("#", "")}`;

  // Shirt silhouette path (viewBox 0 0 100 100):
  // V-neck collar, short sleeves, straight body
  const shirtPath =
    "M30,10 L10,30 L22,35 L22,90 L78,90 L78,35 L90,30 L70,10 L58,20 Q50,26 42,20 Z";

  const textSize = size === "lg" ? 28 : size === "md" ? 20 : 13;
  const textY = size === "lg" ? 70 : size === "md" ? 68 : 68;

  function renderPattern() {
    switch (pattern) {
      case "Stripes":
        return (
          <defs>
            <pattern
              id={`${id}-pat`}
              patternUnits="userSpaceOnUse"
              width="12"
              height="100"
            >
              <rect width="6" height="100" fill={primaryColor} />
              <rect x="6" width="6" height="100" fill={secondaryColor} />
            </pattern>
          </defs>
        );
      case "Hoops":
        return (
          <defs>
            <pattern
              id={`${id}-pat`}
              patternUnits="userSpaceOnUse"
              width="100"
              height="14"
            >
              <rect width="100" height="7" fill={primaryColor} />
              <rect y="7" width="100" height="7" fill={secondaryColor} />
            </pattern>
          </defs>
        );
      default:
        return null;
    }
  }

  function shirtFill() {
    if (pattern === "Stripes" || pattern === "Hoops") {
      return `url(#${id}-pat)`;
    }
    return primaryColor;
  }

  return (
    <svg
      width={px}
      height={px}
      viewBox="0 0 100 100"
      className={className}
      aria-hidden="true"
    >
      {renderPattern()}

      {/* Base shirt */}
      <path d={shirtPath} fill={shirtFill()} />

      {/* Half-and-half overlay */}
      {pattern === "HalfAndHalf" && (
        <>
          <clipPath id={`${id}-left`}>
            <path d={shirtPath} />
          </clipPath>
          <rect
            x="0"
            y="0"
            width="50"
            height="100"
            fill={secondaryColor}
            clipPath={`url(#${id}-left)`}
          />
        </>
      )}

      {/* Diagonal band overlay */}
      {pattern === "Diagonal" && (
        <>
          <clipPath id={`${id}-diag`}>
            <path d={shirtPath} />
          </clipPath>
          <polygon
            points="20,10 80,10 60,90 0,90"
            fill={secondaryColor}
            clipPath={`url(#${id}-diag)`}
          />
        </>
      )}

      {/* Jersey number */}
      {number != null && (
        <text
          x="50"
          y={textY}
          textAnchor="middle"
          dominantBaseline="auto"
          fontSize={textSize}
          fontWeight="bold"
          fontFamily="'Barlow Condensed', 'Inter', sans-serif"
          fill="white"
          stroke="rgba(0,0,0,0.3)"
          strokeWidth="1"
          paintOrder="stroke"
        >
          {number}
        </text>
      )}
    </svg>
  );
}
