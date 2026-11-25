import { ScanSearch } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { PlayerProfileScoutStatus, ScoutAvailability } from "./PlayerProfile.scouting";

interface PlayerProfileScoutActionProps {
  availability: ScoutAvailability;
  scoutStatus: PlayerProfileScoutStatus;
  scoutError: string | null;
  onScout: () => void;
}

export default function PlayerProfileScoutAction({
  availability,
  scoutStatus,
  scoutError,
  onScout,
}: PlayerProfileScoutActionProps) {
  const { t } = useTranslation();

  if (availability.scouts.length === 0) {
    return <p className="text-caption text-fg-muted">{t("scouting.noScoutsHint")}</p>;
  }

  if (availability.alreadyScouting || scoutStatus === "sent") {
    return (
      <span className="text-caption text-positive font-heading font-bold uppercase tracking-wider flex items-center gap-1.5">
        <ScanSearch className="w-3.5 h-3.5" /> {t("scouting.scoutingInProgress")}
      </span>
    );
  }

  return (
    <>
      <button
        type="button"
        disabled={!availability.canScout || scoutStatus === "sending"}
        onClick={onScout}
        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-positive-tint text-positive hover:ring-1 hover:ring-inset hover:ring-positive focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel transition-colors text-caption font-heading font-bold uppercase tracking-wider disabled:opacity-50"
      >
        <ScanSearch className="w-3.5 h-3.5" />
        {scoutStatus === "sending" ? t("scouting.scoutingInProgress") : t("scouting.scoutBtn")}
      </button>
      {scoutError ? <p className="text-caption text-danger mt-1">{scoutError}</p> : null}
    </>
  );
}
