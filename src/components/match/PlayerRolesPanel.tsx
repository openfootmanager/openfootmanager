import { useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { getRoleOptions } from "../../lib/playerRoles";
import { resolveBackendError } from "../../utils/backendI18n";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import { Select } from "../ui";
import { sortByPositionGroup } from "./SubPanel.helpers";
import type { EnginePlayerData, RoleChangeOutcome } from "./types";

/**
 * A role picker for each player on the pitch. The engine is the authority on what a role may be:
 * the list offered is only a convenience, and a role it refuses is shown with its reason while the
 * picker goes back to the role the player still has.
 */
export function PlayerRolesPanel({
  players,
  onChangeRole,
}: {
  players: EnginePlayerData[];
  onChangeRole: (playerId: string, role: string) => Promise<RoleChangeOutcome>;
}) {
  const { t } = useTranslation();
  const alertId = useId();
  // `count` makes every refusal a new alert: the same refusal twice in a row would otherwise change
  // nothing in the DOM, and a screen reader would say nothing the second time.
  const [refusal, setRefusal] = useState<{
    playerId: string;
    player: string;
    reason: string;
    count: number;
  } | null>(null);

  const handleChange = async (player: EnginePlayerData, role: string) => {
    const outcome = await onChangeRole(player.id, role);
    setRefusal((previous) =>
      outcome.applied
        ? null
        : {
            playerId: player.id,
            player: player.name,
            reason: outcome.reason,
            count: (previous?.count ?? 0) + 1,
          },
    );
  };

  return (
    <details
      className="rounded-lg border border-gray-200 bg-white px-3 py-2 dark:border-navy-600 dark:bg-navy-800"
      // A refusal is about the last change made; once the section is put away it is stale.
      onToggle={(event) => {
        if (!event.currentTarget.open) setRefusal(null);
      }}
    >
      <summary className="cursor-pointer rounded font-heading text-xs font-bold uppercase tracking-widest text-gray-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 dark:text-gray-200">
        {t("match.playerRoles")}
      </summary>
      <ul className="mt-2 grid gap-1.5 sm:grid-cols-2">
        {sortByPositionGroup(players).map((player) => (
          <li key={player.id} className="flex items-center justify-between gap-2 text-xs">
            <span className="min-w-0 truncate text-gray-900 dark:text-white">
              <span className="mr-1.5 font-heading text-gray-500 dark:text-gray-400">
                {translatePositionAbbreviation(t, player.position)}
              </span>
              {player.name}
            </span>
            <Select
              value={player.role}
              onChange={(event) => void handleChange(player, event.target.value)}
              aria-label={t("match.roleFor", { player: player.name })}
              aria-describedby={refusal?.playerId === player.id ? alertId : undefined}
              selectSize="xs"
            >
              {getRoleOptions(player.position, player.role).map((role) => (
                <option key={role} value={role}>
                  {t(`tactics.playerRoles.${role}`)}
                </option>
              ))}
            </Select>
          </li>
        ))}
      </ul>
      {refusal && (
        <p
          key={refusal.count}
          id={alertId}
          role="alert"
          className="mt-2 text-xs text-red-600 dark:text-red-400"
        >
          {t("match.roleChangeRefused", {
            player: refusal.player,
            reason: resolveBackendError(refusal.reason),
          })}
        </p>
      )}
    </details>
  );
}
