import { useEffect, useMemo, useState, type ReactNode } from "react";
import { resolveLocalMediaPath } from "../../lib/mediaAssets";
import {
  canGenerateRuntimePlayerPortraits,
  getRuntimeGeneratedPlayerPortrait,
  runtimePortraitIdentityKey,
  type PlayerPortraitIdentity,
} from "../../services/portraitService";
import AssetImage from "./AssetImage";
import GeneratedAvatar from "./GeneratedAvatar";

interface PlayerAvatarPlayer extends PlayerPortraitIdentity {
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
  enableRuntimePortrait?: boolean;
}

function playerInitials(player: PlayerAvatarPlayer): string {
  const source = player.match_name || player.full_name;
  return source.slice(0, 2).toUpperCase();
}

function RuntimePortraitFallback({
  player,
  imageClassName,
  fallback,
}: {
  player: PlayerAvatarPlayer;
  imageClassName: string;
  fallback: ReactNode;
}) {
  const portraitIdentity = useMemo<PlayerPortraitIdentity>(
    () => ({
      id: player.id ?? null,
      full_name: player.full_name,
      match_name: player.match_name,
      nationality: player.nationality ?? null,
      date_of_birth: player.date_of_birth ?? null,
    }),
    [
      player.id,
      player.full_name,
      player.match_name,
      player.nationality,
      player.date_of_birth,
    ],
  );
  const identityKey = useMemo(
    () => runtimePortraitIdentityKey(portraitIdentity),
    [portraitIdentity],
  );
  const [runtimeSrc, setRuntimeSrc] = useState<string | null>(null);
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  const [runtimeImageLoaded, setRuntimeImageLoaded] = useState(false);
  const shouldShowImage = Boolean(runtimeSrc && runtimeSrc !== failedSrc);

  useEffect(() => {
    let cancelled = false;
    setRuntimeSrc(null);
    setFailedSrc(null);
    setRuntimeImageLoaded(false);

    if (!canGenerateRuntimePlayerPortraits()) {
      return () => {
        cancelled = true;
      };
    }

    getRuntimeGeneratedPlayerPortrait(portraitIdentity).then((portrait) => {
      if (!cancelled) {
        setRuntimeSrc(portrait?.imageUrl ?? null);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [identityKey, portraitIdentity]);

  if (shouldShowImage && runtimeSrc) {
    return (
      <div className="relative h-full w-full overflow-hidden">
        <div
          aria-hidden={runtimeImageLoaded ? "true" : undefined}
          className={`h-full w-full transition-opacity duration-200 ease-out ${runtimeImageLoaded ? "opacity-0" : "opacity-100"}`}
        >
          {fallback}
        </div>
        <img
          src={runtimeSrc}
          alt={player.full_name}
          className={`${imageClassName} absolute inset-0 transition-opacity duration-200 ease-out ${runtimeImageLoaded ? "opacity-100" : "opacity-0"}`}
          loading="lazy"
          decoding="async"
          onLoad={() => setRuntimeImageLoaded(true)}
          onError={() => {
            setFailedSrc(runtimeSrc);
            setRuntimeImageLoaded(false);
          }}
        />
      </div>
    );
  }

  return <>{fallback}</>;
}

export function PlayerAvatar({
  player,
  className = "h-9 w-9 shrink-0 overflow-hidden rounded-lg bg-gray-100 dark:bg-navy-700 flex items-center justify-center text-xs font-heading font-bold text-gray-500 dark:text-gray-300",
  imageClassName = "h-full w-full object-cover",
  fallback,
  enableRuntimePortrait = true,
}: PlayerAvatarProps) {
  const defaultFallback =
    fallback ?? (
      <GeneratedAvatar
        name={player.full_name || player.match_name}
        initials={playerInitials(player)}
        className="h-full w-full"
      />
    );

  return (
    <div className={className}>
      <AssetImage
        src={resolveLocalMediaPath(player.media?.face)}
        alt={player.full_name}
        className={imageClassName}
        fallback={
          enableRuntimePortrait ? (
            <RuntimePortraitFallback
              player={player}
              imageClassName="h-full w-full object-contain object-bottom"
              fallback={defaultFallback}
            />
          ) : (
            defaultFallback
          )
        }
      />
    </div>
  );
}
