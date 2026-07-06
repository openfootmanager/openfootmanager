import { useId } from "react";
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
  const uid = useId();
  const id = `jersey-${uid.replace(/:/g, "")}`;

  // Shirt silhouette path (viewBox 0 0 100 100):
  // V-neck collar, short sleeves, straight body
  const shirtPath =
    "M30,10 L10,30 L22,35 L22,90 L78,90 L78,35 L90,30 L70,10 L58,20 Q50,26 42,20 Z";

  const textSize = size === "lg" ? 40 : size === "md" ? 35 : 25;
  const textY = size === "lg" ? 70 : size === "md" ? 68 : 68;

  const usesPattern = pattern === "Stripes" || pattern === "Hoops";
  const usesClip = pattern === "HalfAndHalf" || pattern === "Diagonal";

  return (
    <svg
      width={px}
      height={px}
      viewBox="0 0 100 100"
      className={className}
      aria-hidden="true"
    >
      <defs>
        {pattern === "Stripes" && (
          <pattern id={`${id}-pat`} patternUnits="userSpaceOnUse" width="12" height="100">
            <rect width="6" height="100" fill={primaryColor} />
            <rect x="6" width="6" height="100" fill={secondaryColor} />
          </pattern>
        )}
        {pattern === "Hoops" && (
          <pattern id={`${id}-pat`} patternUnits="userSpaceOnUse" width="100" height="14">
            <rect width="100" height="7" fill={primaryColor} />
            <rect y="7" width="100" height="7" fill={secondaryColor} />
          </pattern>
        )}
        {usesClip && (
          <clipPath id={`${id}-clip`}>
            <path d={shirtPath} />
          </clipPath>
        )}
      </defs>

      {/* Base shirt */}
      <path d={shirtPath} fill={usesPattern ? `url(#${id}-pat)` : primaryColor} />

      {/* Half-and-half: secondary colour on the left half, clipped to shirt shape */}
      {pattern === "HalfAndHalf" && (
        <rect x="0" y="0" width="50" height="100" fill={secondaryColor} clipPath={`url(#${id}-clip)`} />
      )}

      {/* Diagonal band: secondary colour polygon, clipped to shirt shape */}
      {pattern === "Diagonal" && (
        <polygon points="20,10 80,10 60,90 0,90" fill={secondaryColor} clipPath={`url(#${id}-clip)`} />
      )}

      {/* Outline: white line over a soft dark rim so the shirt stays visible
          when the kit colour blends into the background (e.g. green on pitch) */}
      <path
        d={shirtPath}
        fill="none"
        stroke="rgba(0,0,0,0.35)"
        strokeWidth="4"
        strokeLinejoin="round"
      />
      <path
        d={shirtPath}
        fill="none"
        stroke="rgba(255,255,255,0.9)"
        strokeWidth="1.8"
        strokeLinejoin="round"
      />

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
