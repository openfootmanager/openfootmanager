import { Card, CardBody, CardHeader } from "../ui";
import { formatVal } from "../../lib/valueFormatting";
import { buildContractTimeline, type TimelineBar } from "./PlayerProfile.contractTimeline";
import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
import { spanText } from "./PlayerProfileContractHistoryCard";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileContractTimelineProps {
  rows: ContractHistoryRow[];
  /** Whether the club may see his wages: only its own player's contracts name one. */
  showsWages: boolean;
  /** The game's day, as `YYYY-MM-DD`. */
  today: string;
  language: string;
  t: TranslateFn;
}

const SECTION = "playerProfile.contractSheet";

/** The bar's look: the contract in force is tinted, a planned end is dashed, an unknown start is left open. */
function barClassName(bar: TimelineBar): string {
  const fill = bar.inForce ? "border-positive bg-positive-tint" : "border-line-strong bg-sunken";
  const end = bar.planned ? "border-dashed border-r-2" : "";
  const start = bar.startKnown ? "" : "border-l-0 rounded-l-none";
  return `absolute inset-y-0 flex min-w-1 items-center overflow-hidden rounded border px-2 text-caption font-semibold tabular-nums text-fg ${fill} ${end} ${start}`;
}

/**
 * The contracts he has had as bars over time, oldest first: how long each ran, where today falls,
 * and (for his own player) the weekly wage on each. A planned end that has not come yet is dashed.
 * A picture with a name; the list below it says each contract in words.
 */
export default function PlayerProfileContractTimeline({
  rows,
  showsWages,
  today,
  language,
  t,
}: PlayerProfileContractTimelineProps) {
  const timeline = buildContractTimeline(rows, today);
  if (!timeline) return null;
  const rowByKey = new Map(rows.map((row) => [row.key, row]));
  // The label stays on the card at either edge of the axis.
  const todayLabelPct = Math.min(Math.max(timeline.todayPct, 5), 95);

  return (
    <Card>
      <CardHeader>{t(`${SECTION}.overTime`)}</CardHeader>
      <CardBody className="space-y-2">
        <div role="img" aria-label={t(`${SECTION}.overTimeLabel`)} className="space-y-1">
          <div className="relative h-4">
            <span
              className="absolute -translate-x-1/2 text-caption font-semibold text-info"
              style={{ left: `${todayLabelPct}%` }}
            >
              {t(`${SECTION}.today`)}
            </span>
          </div>
          <div className="relative h-9 rounded bg-sunken/40">
            {timeline.bars.map((bar) => {
              const row = rowByKey.get(bar.key);
              const title = `${bar.clubName} · ${row ? spanText(row, language, t) : ""}`;
              return (
                <div
                  key={bar.key}
                  title={title}
                  className={barClassName(bar)}
                  style={{ left: `${bar.leftPct}%`, width: `${bar.widthPct}%` }}
                >
                  {showsWages ? (
                    <span className="truncate">{formatVal(bar.weeklyWage)}</span>
                  ) : null}
                </div>
              );
            })}
            <span
              aria-hidden="true"
              className="absolute inset-y-0 w-0.5 -translate-x-1/2 bg-info"
              style={{ left: `${timeline.todayPct}%` }}
            />
          </div>
          <div className="relative h-4 border-t border-line">
            {timeline.ticks.map((tick) => (
              <span
                key={tick.year}
                className="absolute text-caption tabular-nums text-fg-muted"
                style={{ left: `${tick.leftPct}%` }}
              >
                {tick.year}
              </span>
            ))}
          </div>
        </div>
        <p className="text-caption text-fg-muted">
          {t(showsWages ? `${SECTION}.overTimeNote` : `${SECTION}.overTimeNoteNoWage`)}
        </p>
      </CardBody>
    </Card>
  );
}
