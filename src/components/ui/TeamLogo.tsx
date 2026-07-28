import type { CSSProperties, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { usePackageAssetSrc } from "../../hooks/usePackageAssetSrc";
import AssetImage from "./AssetImage";
import GeneratedCrest from "./GeneratedCrest";

interface TeamLogoTeam {
  name: string;
  short_name: string;
  colors?: {
    primary?: string;
    secondary?: string;
  };
  media?: {
    logo?: string | null;
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
  const { t } = useTranslation();
  const logoSrc = usePackageAssetSrc(team.media?.logo);

  return (
    <div className={className} style={style}>
      <AssetImage
        src={logoSrc}
        alt={t("common.teamLogoAlt", { team: team.name })}
        className={imageClassName}
        fallback={
          fallback ?? (
            <GeneratedCrest
              name={team.name}
              label={team.short_name}
              colors={team.colors}
              className="h-full w-full"
            />
          )
        }
      />
    </div>
  );
}
