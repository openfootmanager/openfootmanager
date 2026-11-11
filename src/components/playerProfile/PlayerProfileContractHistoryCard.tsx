import { Badge, Card, CardBody, CardHeader } from "../ui";
import type { ReleaseReason } from "../../store/types";
import { formatPlayerWage, formatProfileDay } from "./PlayerProfile.helpers";
import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileContractHistoryCardProps {
  rows: ContractHistoryRow[];
  /** Whether the club may see his wages: only its own player's contracts name one. */
  showsWages: boolean;
  /** The language dates are written in. */
  language: string;
  /** Opens a club by its id; without it the club is plain text. */
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}

const RELEASE_REASON_KEYS: Record<ReleaseReason, string> = {
  expired: "playerProfile.movementReleasedExpired",
  terminated: "playerProfile.movementReleasedTerminated",
};

/** The contract's span in words. An unknown start is left out, never replaced by a date. */
function spanText(row: ContractHistoryRow, language: string, t: TranslateFn): string {
  const start = row.start ? formatProfileDay(row.start, language) : null;
  const end = row.end ? formatProfileDay(row.end, language) : null;
  if (start && end) return t("playerProfile.movementContractRange", { start, end });
  if (end) return t("playerProfile.movementContractUntil", { end });
  if (start) return t("playerProfile.movementContractFrom", { start });
  return t("playerProfile.movementDateUnknown");
}

/** The contracts he has had, newest first, from his movement ledger. */
export default function PlayerProfileContractHistoryCard({
  rows,
  showsWages,
  language,
  onOpenTeam: _onOpenTeam,
  t,
}: PlayerProfileContractHistoryCardProps) {
  return (
    <Card>
      <CardHeader>{t("playerProfile.contractsHad")}</CardHeader>
      <CardBody>
        {rows.length > 0 ? (
          <table className="w-full text-body">
            <thead>
              <tr className="border-b border-line text-left text-caption text-fg-muted">
                <th scope="col" className="pb-2 pr-3 font-semibold">
                  {t("common.team")}
                </th>
                <th scope="col" className="pb-2 pr-3 font-semibold">
                  {t("playerProfile.contractSpanHeading")}
                </th>
                {showsWages ? (
                  <th scope="col" className="pb-2 pr-3 text-right font-semibold">
                    {t("playerProfile.contractSheet.weeklyWage")}
                  </th>
                ) : null}
                <th scope="col" className="pb-2 font-semibold">
                  {t("playerProfile.contractEnded")}
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-line">
              {rows.map((row) => (
                <tr key={row.key}>
                  <td className="py-2 pr-3 font-semibold text-fg">{row.clubName}</td>
                  <td className="py-2 pr-3 text-fg-muted">
                    <span>{spanText(row, language, t)}</span>
                    {row.source === "legacy_migrated" ? (
                      <span className="block text-caption text-fg-subtle">
                        {t("playerProfile.movementLegacyContract")}
                      </span>
                    ) : null}
                  </td>
                  {showsWages ? (
                    <td className="py-2 pr-3 text-right tabular-nums text-fg">
                      {formatPlayerWage(row.weeklyWage)}
                    </td>
                  ) : null}
                  <td className="py-2 text-fg-muted">
                    {row.inForce ? (
                      <Badge variant="success">{t("playerProfile.contractCurrent")}</Badge>
                    ) : row.endedBy ? (
                      t(RELEASE_REASON_KEYS[row.endedBy])
                    ) : null}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className="py-4 text-center text-body text-fg-subtle">
            {t("playerProfile.noContractsHad")}
          </p>
        )}
      </CardBody>
    </Card>
  );
}
