import type { ReactNode } from "react";
import { resolveLocalMediaPath } from "../../lib/mediaAssets";
import AssetImage from "./AssetImage";
import GeneratedAvatar from "./GeneratedAvatar";

interface PlayerAvatarPlayer {
  full_name: string;
  match_name: string;
  media?: {
    face?: string;
  };
}

interface PlayerAvatarProps {
  player: PlayerAvatarPlayer;
  className?: string;
  imageClassName?: string;
  fallback?: ReactNode;
}

function playerInitials(player: PlayerAvatarPlayer): string {
  const source = player.match_name || player.full_name;
  return source.slice(0, 2).toUpperCase();
}

export function PlayerAvatar({
  player,
  className = "h-9 w-9 shrink-0 overflow-hidden rounded-lg bg-gray-100 dark:bg-navy-700 flex items-center justify-center text-xs font-heading font-bold text-gray-500 dark:text-gray-300",
  imageClassName = "h-full w-full object-cover",
  fallback,
}: PlayerAvatarProps) {
  return (
    <div className={className}>
      <AssetImage
        src={resolveLocalMediaPath(player.media?.face)}
        alt={player.full_name}
        className={imageClassName}
        fallback={
          fallback ?? (
            <GeneratedAvatar
              name={player.full_name || player.match_name}
              initials={playerInitials(player)}
              className="h-full w-full"
            />
          )
        }
      />
    </div>
  );
}
