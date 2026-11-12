import { useId, useState } from "react";

import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import { SegmentedControl } from "../ui";
import { buildCareerChart, type ChartBar, type ChartMetric } from "./PlayerProfile.careerChart";
import { ClubName, moveSummary, seasonLabel, type TranslateFn } from "./careerParts";

interface PlayerProfileCareerChartProps {
  career: PlayerCareer;
  language: string;
  /** Opens a club by its id; without it the club is plain text. */
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}

const SECTION = "playerProfile";

/** One fill for each club band, in the order clubs appear; the legend names them, so colour is never the only signal. */
const BAND_FILLS = [
  "bg-warning-solid",
  "bg-info-solid",
  "bg-positive-solid",
  "bg-danger-solid",
  "bg-fg-subtle",
] as const;
const bandFill = (index: number) => BAND_FILLS[index % BAND_FILLS.length];

const METRIC_KEYS: Record<ChartMetric, string> = {
  goals: `${SECTION}.goals`,
  assists: `${SECTION}.assists`,
  appearances: `${SECTION}.apps`,
};

function Readout({
  bar,
  language,
  onOpenTeam,
  t,
}: {
  bar: ChartBar;
  language: string;
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}) {
  const figures: [string, number][] = [
    [t(`${SECTION}.apps`), bar.appearances],
    [t(`${SECTION}.goals`), bar.goals],
    [t(`${SECTION}.assists`), bar.assists],
  ];
  return (
    <section
      aria-label={t(`${SECTION}.careerChartReadout`)}
      className="flex flex-col gap-2 rounded-lg border border-line bg-sunken p-4"
    >
      <p className="font-heading text-title font-bold tabular-nums text-fg">
        {seasonLabel(bar.season)}
      </p>
      <p>
        <ClubName id={bar.clubId} name={bar.clubName} onOpenTeam={onOpenTeam} t={t} />
      </p>
      <dl className="grid grid-cols-3 gap-2">
        {figures.map(([label, value]) => (
          <div key={label}>
            <dt className="text-caption text-fg-muted">{label}</dt>
            <dd className="font-heading text-lead font-bold tabular-nums text-fg">{value}</dd>
          </div>
        ))}
      </dl>
      <p className="text-caption text-fg-muted">
        {bar.move ? moveSummary(bar.move, language, t) : t(`${SECTION}.careerChartNoMove`)}
      </p>
    </section>
  );
}

/**
 * The career as a bar for each season (oldest left), a Goals / Assists / Appearances switch, a band
 * under each bar for its club (a loan is a band of its own), and a readout that follows the chosen
 * season. Every bar is a button, so the chart is reached by keyboard and named for a screen reader.
 */
export default function PlayerProfileCareerChart({
  career,
  language,
  onOpenTeam,
  t,
}: PlayerProfileCareerChartProps) {
  const titleId = useId();
  const [metric, setMetric] = useState<ChartMetric>("goals");
  const [chosen, setChosen] = useState<string | null>(null);
  const { bars, bands } = buildCareerChart(career, metric);
  // The latest season until another is chosen, and again if the chosen one is no longer there.
  const selected = bars.find((bar) => bar.key === chosen) ?? bars[bars.length - 1];
  if (!selected) return null;
  const metricLabel = t(METRIC_KEYS[metric]);

  return (
    <div className="flex flex-col gap-4 p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <span id={titleId} className="text-caption font-semibold text-fg-muted">
          {t(`${SECTION}.careerChartTitle`)}
        </span>
        <SegmentedControl
          labelledBy={titleId}
          options={(Object.keys(METRIC_KEYS) as ChartMetric[]).map((value) => ({
            value,
            label: t(METRIC_KEYS[value]),
          }))}
          value={metric}
          onChange={setMetric}
        />
      </div>
      <div className="grid grid-cols-1 gap-4 @[40rem]:grid-cols-[minmax(0,1fr)_14rem]">
        <div className="flex min-w-0 flex-col gap-3">
          <div className="flex items-end gap-2 overflow-x-auto">
            {bars.map((bar) => {
              const isSelected = bar.key === selected.key;
              return (
                <div key={bar.key} className="flex min-w-10 flex-1 flex-col items-center gap-1">
                  <button
                    type="button"
                    aria-pressed={isSelected}
                    aria-label={t(`${SECTION}.careerChartBar`, {
                      season: seasonLabel(bar.season),
                      club: bar.clubName,
                      value: bar.value,
                      metric: metricLabel,
                    })}
                    onClick={() => setChosen(bar.key)}
                    className={`flex w-full flex-col items-center gap-1 rounded focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel ${isSelected ? "bg-selected-tint" : ""}`}
                  >
                    <span className="text-caption font-bold tabular-nums text-fg">{bar.value}</span>
                    <span className="flex h-36 w-full items-end justify-center">
                      <span
                        role="presentation"
                        className={`w-full max-w-10 rounded-t ${isSelected ? "bg-info-solid" : "bg-fg-subtle"}`}
                        style={{ height: `${bar.heightPct}%` }}
                      />
                    </span>
                  </button>
                  <span className="text-caption tabular-nums text-fg-muted">
                    {seasonLabel(bar.season)}
                  </span>
                  <span
                    aria-hidden="true"
                    className={`h-1.5 w-full rounded-full ${bandFill(bar.bandIndex)}`}
                  />
                </div>
              );
            })}
          </div>
          <ul
            aria-label={t(`${SECTION}.careerChartClubs`)}
            className="flex flex-wrap gap-x-4 gap-y-1 text-caption text-fg-muted"
          >
            {bands.map((band) => (
              <li key={band.key} className="inline-flex items-center gap-1.5">
                <span
                  aria-hidden="true"
                  className={`h-1.5 w-4 rounded-full ${bandFill(band.bandIndex)}`}
                />
                {band.onLoan
                  ? t(`${SECTION}.careerChartLoanClub`, { club: band.clubName })
                  : band.clubName}
              </li>
            ))}
          </ul>
        </div>
        <Readout bar={selected} language={language} onOpenTeam={onOpenTeam} t={t} />
      </div>
    </div>
  );
}
