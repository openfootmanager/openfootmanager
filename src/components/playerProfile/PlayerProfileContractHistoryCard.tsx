import { Card, CardBody, CardHeader } from "../ui";
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

const SECTION = "playerProfile.contractSheet";

const RELEASE_REASON_KEYS: Record<ReleaseReason | "retired", string> = {
  expired: "playerProfile.movementReleasedExpired",
  terminated: "playerProfile.movementReleasedTerminated",
  retired: "playerProfile.movementRetired",
};

/** The contract's span in words. An unknown start is left out, never replaced by a date. */
export function spanText(row: ContractHistoryRow, language: string, t: TranslateFn): string {
  const start = row.start ? formatProfileDay(row.start, language) : null;
  const end = row.end ? formatProfileDay(row.end, language) : null;
  if (start && end) return t(`${SECTION}.range`, { start, end });
  if (end) return t(`${SECTION}.until`, { end });
  if (start) return t(`${SECTION}.from`, { start });
  return t("playerProfile.movementDateUnknown");
}

/** How he came to the club: a transfer names the club he came from; a legacy contract says it is one. */
function howText(row: ContractHistoryRow, t: TranslateFn): string {
  if (row.source === "legacy_migrated") return t("playerProfile.movementLegacyContract");
  if (row.source === "transfer") {
    return row.fromClubName
      ? t(`${SECTION}.how.transfer`, { club: row.fromClubName })
      : t(`${SECTION}.how.transferNoClub`);
  }
  return t(`${SECTION}.how.${row.source}`);
}

/**
 * The contracts he has had, newest first, from his movement ledger: each row leads with its date
 * range, then the club (a link) with how he joined it, then the wage; the contract he is on now is
 * tinted, and one that ended says how.
 */
export default function PlayerProfileContractHistoryCard({
  rows,
  showsWages,
  language,
  onOpenTeam,
  t,
}: PlayerProfileContractHistoryCardProps) {
  return (
    <Card>
      <CardHeader>{t("playerProfile.contractsHad")}</CardHeader>
      <CardBody>
        {rows.length > 0 ? (
          <table className="w-full text-body">
            <thead className="sr-only">
              <tr>
                <th scope="col">{t("playerProfile.contractSpanHeading")}</th>
                <th scope="col">{t("common.team")}</th>
                {showsWages ? <th scope="col">{t(`${SECTION}.weeklyWage`)}</th> : null}
              </tr>
            </thead>
            <tbody className="divide-y divide-line">
              {rows.map((row) => {
                return (
                  <tr key={row.key} className={row.inForce ? "bg-positive-tint" : undefined}>
                    <td className="py-2 pr-3 align-top font-semibold tabular-nums text-fg">
                      {spanText(row, language, t)}
                    </td>
                    <td className="min-w-0 py-2 pr-3 align-top">
                      {onOpenTeam && row.clubId ? (
                        <button
                          type="button"
                          onClick={() => onOpenTeam(row.clubId ?? "")}
                          className="rounded font-semibold text-positive underline underline-offset-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
                        >
                          {row.clubName}
                        </button>
                      ) : (
                        <span className="font-semibold text-fg">{row.clubName}</span>
                      )}
                      <Note>{howText(row, t)}</Note>
                      {!row.inForce && row.endedBy ? (
                        <Note>{t(RELEASE_REASON_KEYS[row.endedBy])}</Note>
                      ) : null}
                      {row.replacedOn ? (
                        <Note>
                          {t(`${SECTION}.replacedOn`, {
                            date: formatProfileDay(row.replacedOn, language),
                          })}
                        </Note>
                      ) : null}
                    </td>
                    {showsWages ? (
                      <td className="py-2 text-right align-top font-semibold tabular-nums text-fg">
                        {formatPlayerWage(row.weeklyWage)}
                      </td>
                    ) : null}
                  </tr>
                );
              })}
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

/** A muted note after the club's name, set off from it by a dot. */
function Note({ children }: { children: string }) {
  return (
    <>
      <span aria-hidden="true" className="text-caption text-fg-muted">
        {" · "}
      </span>
      <span className="text-caption text-fg-muted">{children}</span>
    </>
  );
}
