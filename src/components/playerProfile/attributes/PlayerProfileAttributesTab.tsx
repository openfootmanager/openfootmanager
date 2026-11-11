import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import type { KnowledgeAnswer } from "../../../hooks/useKnowledgeAnswer";
import { useRatingTiers } from "../../../hooks/useRatingTiers";
import { formatDate } from "../../../lib/dateFormatting";
import { ratingTierClass } from "../../../lib/ratingTier";
import type { PositionRatings } from "../../../store/generated/PositionRatings";
import type { PlayerData } from "../../../store/gameStore";
import { Badge } from "../../ui";
import ProfileLoadFailed from "../ProfileLoadFailed";
import { type AttributeRow, attributeColumns } from "./attributesTabModel";

export interface PlayerProfileAttributesTabProps {
  /** The attributes the club may read (a scout's report for a scouted player); null when none. */
  attributes: PlayerData["attributes"] | null;
  isGoalkeeper: boolean;
  /** His ratings by position: the natural position's weights and key attributes set the dots and the highlight. */
  ratings: KnowledgeAnswer<PositionRatings | null>;
  /** The date of the scout's report the attributes are read from; null for the club's own view. */
  scoutedOn: string | null;
  scoutOutOfDate: boolean;
  language: string;
  /** The club's opinion of his potential, or its load-failed state, drawn in the footer. */
  potentialOpinion: ReactNode;
}

/**
 * The Attributes tab, laid out as the approved board: his attributes in Technical, Mental and
 * Physical columns (Goalkeeping for a keeper), each row with the dots for how much the game weighs it
 * at his natural position and a bar and value in the backend's tier tone, the position's key
 * attributes highlighted, and the potential opinion and the scouted date in the footer. No position
 * cards: his ratings by position are not repeated here.
 */
export default function PlayerProfileAttributesTab(props: PlayerProfileAttributesTabProps) {
  const { t } = useTranslation();
  const { attributes } = props;
  if (!attributes) {
    return (
      <section
        aria-label={t("playerProfile.attributes")}
        className="relative rounded-xl border border-line bg-panel px-4 py-3 shadow-card"
      >
        <h3 className="font-heading text-body font-bold uppercase tracking-label text-fg">
          {t("playerProfile.attributes")}
        </h3>
        <p className="mt-2 font-semibold text-fg">{t("playerProfile.attributesHidden")}</p>
        <p className="text-body text-fg-muted">{t("playerProfile.scoutToView")}</p>
        {props.potentialOpinion}
      </section>
    );
  }
  return <ReadableAttributes {...props} attributes={attributes} />;
}

function ReadableAttributes({
  attributes,
  isGoalkeeper,
  ratings,
  scoutedOn,
  scoutOutOfDate,
  language,
  potentialOpinion,
}: PlayerProfileAttributesTabProps & { attributes: PlayerData["attributes"] }) {
  const { t } = useTranslation();
  const tierOf = useRatingTiers();
  const natural = ratings.status === "answered" ? ratings.value?.rows[0] : undefined;
  // How much each attribute counts at his natural position, in the backend's three steps; an
  // attribute the position does not weigh is not in the list and has none.
  const importance = new Map<string, number>(
    (natural?.weights ?? []).map(({ attribute, importance }) => [attribute, importance]),
  );
  const keyAttributes = new Set<string>(
    (natural?.key_attributes ?? []).map((key) => key.attribute),
  );
  const columns = attributeColumns(attributes, isGoalkeeper, t);

  return (
    <section
      aria-label={t("playerProfile.attributes")}
      className="relative flex flex-col overflow-hidden rounded-xl border border-line bg-panel shadow-card"
    >
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1 border-b border-line px-4 py-2.5">
        <h3 className="font-heading text-body font-bold uppercase tracking-label text-fg">
          {t("playerProfile.attributes")}
        </h3>
        <div className="text-caption text-fg-muted">
          {ratings.status === "failed" ? <ProfileLoadFailed t={t} onRetry={ratings.retry} /> : null}
          {ratings.status === "loading" ? t("common.loading") : null}
        </div>
      </div>
      <div className="@container">
        <div data-columns className="grid grid-cols-1 @[56rem]:grid-cols-3">
          {columns.map((column) => (
            <div
              key={column.label}
              className="flex min-w-0 flex-col border-line @[56rem]:border-r last:border-r-0"
            >
              <div className="flex items-center justify-between border-b border-line bg-sunken px-3.5 py-1.5">
                <h4 className="font-heading text-body font-bold uppercase tracking-label text-fg-muted">
                  {column.label}
                </h4>
                <span className="text-caption tabular-nums text-fg-muted">
                  {t("playerProfile.attributesTab.average", { value: column.average })}
                </span>
              </div>
              {/* biome-ignore lint/a11y/noRedundantRoles: WebKit drops a list's semantics under list-style: none, so the role is restated. */}
              <ul role="list" className="m-0 list-none p-0">
                {column.attrs.map((attr) => (
                  <AttributeLine
                    key={attr.key}
                    attr={attr}
                    dots={natural ? clampDots(importance.get(attr.key) ?? 0) : null}
                    isKey={keyAttributes.has(attr.key)}
                    tier={tierOf(attr.value)}
                  />
                ))}
              </ul>
            </div>
          ))}
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-x-6 gap-y-2 border-t border-line bg-sunken px-4 py-2.5">
        <div className="min-w-0 flex-1">{potentialOpinion}</div>
        <p className="flex items-center gap-2 text-body text-fg">
          <span className="text-fg-muted">{t("playerProfile.attributesTab.scouted")}</span>
          <span className="font-semibold">{scoutedOn ? formatDate(scoutedOn, language) : "–"}</span>
          {scoutedOn && scoutOutOfDate ? (
            <Badge variant="accent">{t("playerProfile.scoutReportOutOfDate")}</Badge>
          ) : null}
        </p>
      </div>
    </section>
  );
}

/** The backend's importance as the three dots drawn: 0 to 3, nothing else. */
function clampDots(importance: number): 0 | 1 | 2 | 3 {
  return Math.max(0, Math.min(3, Math.round(importance))) as 0 | 1 | 2 | 3;
}

function AttributeLine({
  attr,
  dots,
  isKey,
  tier,
}: {
  attr: AttributeRow;
  /** Null when his position's weights are not known: no dots at all. */
  dots: 0 | 1 | 2 | 3 | null;
  isKey: boolean;
  tier: ReturnType<ReturnType<typeof useRatingTiers>>;
}) {
  const { t } = useTranslation();
  return (
    <li
      data-key={isKey ? "true" : undefined}
      className={`grid grid-cols-[minmax(0,1fr)_auto_minmax(3rem,5rem)_2rem] items-center gap-3 border-b border-line px-3.5 py-1.5 text-body last:border-b-0 ${
        isKey ? "bg-selected-tint font-semibold" : ""
      }`}
    >
      <span className="truncate">
        {attr.name}
        {isKey ? <span className="sr-only">{t("playerProfile.attributesTab.key")}</span> : null}
      </span>
      {dots === null ? (
        <span aria-hidden="true" className="w-[1.375rem]" />
      ) : (
        <span
          role="img"
          aria-label={t("playerProfile.attributesTab.weight", { count: dots })}
          className="flex gap-1"
        >
          {[1, 2, 3].map((step) => (
            <span
              key={step}
              className={`size-1.5 rounded-full ${step <= dots ? "bg-fg-muted" : "bg-line"}`}
            />
          ))}
        </span>
      )}
      <span className="h-1.5 overflow-hidden rounded-full bg-sunken">
        <span
          className={`block h-full ${ratingTierClass(tier, "bar")}`}
          style={{ width: `${Math.max(0, Math.min(100, attr.value))}%` }}
        />
      </span>
      <span
        data-value
        className={`text-right font-bold tabular-nums ${ratingTierClass(tier, "text")}`}
      >
        {attr.value}
      </span>
    </li>
  );
}
