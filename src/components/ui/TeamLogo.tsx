import type { CSSProperties, ReactNode } from "react";
import { resolveLocalMediaPath } from "../../lib/mediaAssets";
import AssetImage from "./AssetImage";

interface TeamLogoTeam {
  name: string;
  short_name: string;
  media?: {
    logo?: string;
  };
}

interface TeamLogoProps {
  team: TeamLogoTeam;
  className?: string;
  imageClassName?: string;
  fallback?: ReactNode;
  style?: CSSProperties;
}

export function TeamLogo({
  team,
  className = "h-12 w-12 shrink-0 overflow-hidden rounded-lg bg-white/10 flex items-center justify-center font-heading font-bold text-lg text-white",
  imageClassName = "h-10 w-10 object-contain drop-shadow",
  fallback,
  style,
}: TeamLogoProps) {
  return (
    <div className={className} style={style}>
      <AssetImage
        src={resolveLocalMediaPath(team.media?.logo)}
        alt={`${team.name} logo`}
        className={imageClassName}
        fallback={fallback ?? <span>{team.short_name}</span>}
      />
    </div>
  );
}
