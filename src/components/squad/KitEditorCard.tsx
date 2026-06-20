import { useState } from "react";
import { useTranslation } from "react-i18next";
import JerseyIcon from "../ui/JerseyIcon";
import { setTeamKitPattern } from "../../services/squadService";
import type { GameStateData } from "../../store/gameStore";
import type { KitPattern } from "../../store/types";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";

const KIT_PATTERNS: KitPattern[] = [
  "Solid",
  "Stripes",
  "Hoops",
  "HalfAndHalf",
  "Diagonal",
];

interface KitEditorCardProps {
  primaryColor: string;
  secondaryColor: string;
  currentPattern: KitPattern;
  onGameUpdate?: (g: GameStateData) => void;
}

export default function KitEditorCard({
  primaryColor,
  secondaryColor,
  currentPattern,
  onGameUpdate,
}: KitEditorCardProps) {
  const { t } = useTranslation();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSelectPattern(pattern: KitPattern) {
    if (pattern === currentPattern || saving) return;
    setSaving(true);
    setError(null);
    try {
      const updated = await setTeamKitPattern(pattern);
      onGameUpdate?.(updated);
    } catch (err) {
      setError(resolveTranslatedErrorMessage(err, t));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex flex-col gap-3 p-4 rounded-xl bg-white dark:bg-navy-800 border border-gray-100 dark:border-navy-700">
      <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
        {t("squad.teamKit")}
      </p>
      <div className="flex items-center gap-3 flex-wrap">
        {KIT_PATTERNS.map((pat) => {
          const selected = pat === currentPattern;
          return (
            <button
              key={pat}
              type="button"
              title={t(`squad.kitPatterns.${pat}`)}
              onClick={() => handleSelectPattern(pat)}
              disabled={saving}
              className={`flex flex-col items-center gap-1 rounded-lg p-1.5 transition-all
                          focus:outline-none focus:ring-2 focus:ring-primary-500/40
                          ${selected
                  ? "ring-2 ring-primary-500 bg-primary-500/10"
                  : "hover:bg-gray-100 dark:hover:bg-navy-700"
                }`}
            >
              <JerseyIcon
                primaryColor={primaryColor}
                secondaryColor={secondaryColor}
                pattern={pat}
                size="md"
              />
              <span className="text-xs text-gray-500 dark:text-gray-400 font-heading">
                {t(`squad.kitPatterns.${pat}`)}
              </span>
            </button>
          );
        })}
      </div>
      {error && (
        <p className="text-xs text-red-500 dark:text-red-400">{error}</p>
      )}
    </div>
  );
}
