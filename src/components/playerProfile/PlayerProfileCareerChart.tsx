import { useId, useState } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  LabelList,
  ResponsiveContainer,
  XAxis,
  type XAxisTickContentProps,
  YAxis,
} from "recharts";

import type { CareerMoveLine } from "../../store/generated/CareerMoveLine";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import { storedTeamLabel } from "../../lib/team";
import { Badge, Button, SegmentedControl } from "../ui";
import { ChartContainer } from "../ui/charts/ChartContainer";
import { useChartTheme } from "../ui/charts/chartTheme";
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

/**
 * One fill for each club band, in the order clubs appear, as the role-token variables an SVG
 * resolves; the legend names each club, so colour is never the only signal.
 */
const BAND_FILLS = [
  "var(--color-warning-solid)",
  "var(--color-info-solid)",
  "var(--color-positive-solid)",
  "var(--color-danger-solid)",
  "var(--color-fg-subtle)",
] as const;
const bandFill = (index: number) => BAND_FILLS[index % BAND_FILLS.length];

const METRIC_KEYS: Record<ChartMetric, string> = {
  goals: `${SECTION}.goals`,
  assists: `${SECTION}.assists`,
  appearances: `${SECTION}.apps`,
};

function Readout({
  bar,
  isFirst,
  after,
  language,
  onOpenTeam,
  t,
}: {
  bar: ChartBar;
  /** The first bar of the career: its moves are counted from joining, not from a previous spell. */
  isFirst: boolean;
  /** The moves after the career's last season, for the latest bar only. */
  after: CareerMoveLine[];
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
      aria-live="polite"
      className="flex flex-col gap-2 rounded-lg border border-line bg-sunken p-4"
    >
      <p className="font-heading text-title font-bold tabular-nums text-fg">
        {seasonLabel(bar.season)}
      </p>
      <p className="flex flex-wrap items-center gap-2">
        <ClubName id={bar.clubId} name={bar.clubName} onOpenTeam={onOpenTeam} t={t} />
        {bar.inProgress ? (
          <Badge variant="success">{t(`${SECTION}.careerInProgress`)}</Badge>
        ) : null}
        {bar.onLoan ? <Badge variant="neutral">{t(`${SECTION}.careerOnLoan`)}</Badge> : null}
      </p>
      <dl className="grid grid-cols-3 gap-2">
        {figures.map(([label, value]) => (
          <div key={label}>
            <dt className="text-caption text-fg-muted">{label}</dt>
            <dd className="font-heading text-lead font-bold tabular-nums text-fg">{value}</dd>
          </div>
        ))}
      </dl>
      {bar.movesBefore.length > 0 ? (
        <div className="text-caption text-fg-muted">
          <p className="font-semibold">
            {t(`${SECTION}.careerChartMoves${isFirst ? "First" : "Since"}`)}
          </p>
          <ul className="flex flex-col gap-1">
            {bar.movesBefore.map((move) => (
              <li key={move.key}>{moveSummary(move, language, t)}</li>
            ))}
          </ul>
        </div>
      ) : (
        <p className="text-caption text-fg-muted">
          {t(`${SECTION}.careerChartNoMove${isFirst ? "First" : "Since"}`)}
        </p>
      )}
      {after.length > 0 ? (
        <div className="text-caption text-fg-muted">
          <p className="font-semibold">{t(`${SECTION}.careerChartMovesAfter`)}</p>
          <ul className="flex flex-col gap-1">
            {after.map((move) => (
              <li key={move.key}>{moveSummary(move, language, t)}</li>
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  );
}

/** The key of the datum behind a clicked bar: recharts hands the bar's rectangle, the datum as `payload`. */
function datumKey(payload: unknown): string | null {
  if (typeof payload !== "object" || payload === null || !("key" in payload)) return null;
  return typeof payload.key === "string" ? payload.key : null;
}

/**
 * The career as a bar for each season (oldest left), a Goals / Assists / Appearances switch, a band
 * under each bar's season for its club (a loan is a band of its own), and a readout that follows the
 * chosen season. Drawn by recharts like the other charts; its bars are not focusable, so previous
 * and next season buttons step the chosen season by keyboard and the readout announces it.
 */
export default function PlayerProfileCareerChart({
  career,
  language,
  onOpenTeam,
  t,
}: PlayerProfileCareerChartProps) {
  const titleId = useId();
  const theme = useChartTheme();
  const [metric, setMetric] = useState<ChartMetric>("goals");
  const [chosen, setChosen] = useState<string | null>(null);
  const { bars, bands, trailingMoves } = buildCareerChart(career, metric);
  // The latest season until another is chosen, and again if the chosen one is no longer there.
  const found = bars.findIndex((bar) => bar.key === chosen);
  const index = found >= 0 ? found : bars.length - 1;
  const selected = bars[index];
  if (!selected) {
    return (
      <div className="p-6">
        <ChartContainer isEmpty height={160} />
      </div>
    );
  }

  const data = bars.map((bar) => ({
    key: bar.key,
    label: seasonLabel(bar.season),
    value: bar.value,
    bandIndex: bar.bandIndex,
  }));
  // A bar's tick is its season with its club's band under it.
  // The tick position is a number or a numeric string; the season label is the datum's.
  const renderTick = ({ x, y, index: at, payload }: XAxisTickContentProps) => {
    const left = Number(x);
    const top = Number(y);
    return (
      <g>
        <text x={left} y={top + 12} textAnchor="middle" fill={theme.axisColor}>
          {String(payload.value)}
        </text>
        <rect
          x={left - 14}
          y={top + 20}
          width={28}
          height={5}
          rx={2.5}
          fill={bandFill(data[at]?.bandIndex ?? 0)}
        />
        <text x={left} y={top + 38} textAnchor="middle" fill={theme.axisColor}>
          {(data[at]?.bandIndex ?? 0) + 1}
        </text>
      </g>
    );
  };
  const choose = (payload: unknown) => setChosen(datumKey(payload));
  const step = (by: number) => setChosen(bars[index + by]?.key ?? selected.key);

  return (
    <div className="@container flex flex-col gap-4 p-6">
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
          {/* The chart's text takes the role size from here: no size is written into the drawing. */}
          <div className="overflow-x-auto">
            <div style={{ minWidth: `${bars.length * 3.5}rem` }}>
              <ChartContainer height={240} className="text-caption">
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={data} margin={{ top: 16, right: 8, bottom: 8, left: 0 }}>
                    <CartesianGrid
                      stroke={theme.gridColor}
                      strokeDasharray="3 3"
                      vertical={false}
                    />
                    <XAxis
                      dataKey="label"
                      tick={renderTick}
                      height={52}
                      axisLine={{ stroke: theme.gridColor }}
                      tickLine={false}
                      interval={0}
                    />
                    <YAxis
                      allowDecimals={false}
                      tick={{ fill: theme.axisColor }}
                      axisLine={false}
                      tickLine={false}
                      width={28}
                    />
                    <Bar
                      dataKey="value"
                      radius={[3, 3, 0, 0]}
                      minPointSize={3}
                      onClick={(item) => choose(item.payload)}
                      isAnimationActive={false}
                    >
                      {data.map((datum, at) => (
                        <Cell
                          key={datum.key}
                          fill={at === index ? theme.primary : theme.axisColor}
                          cursor="pointer"
                        />
                      ))}
                      <LabelList dataKey="value" position="top" fill={theme.textColor} />
                    </Bar>
                  </BarChart>
                </ResponsiveContainer>
              </ChartContainer>
            </div>
          </div>
          <div className="flex items-center justify-between gap-3">
            <ul
              aria-label={t(`${SECTION}.careerChartClubs`)}
              className="flex flex-wrap gap-x-4 gap-y-1 text-caption text-fg-muted"
            >
              {bands.map((band) => {
                const label = storedTeamLabel(band.clubName, t);
                return (
                  <li key={band.key} className="inline-flex items-center gap-1.5">
                    <span
                      aria-hidden="true"
                      className="h-1.5 w-4 rounded-full"
                      style={{ backgroundColor: bandFill(band.bandIndex) }}
                    />
                    <span className="font-semibold tabular-nums">{band.bandIndex + 1}</span>{" "}
                    {band.onLoan ? t(`${SECTION}.careerChartLoanClub`, { club: label }) : label}
                  </li>
                );
              })}
            </ul>
            <div className="flex shrink-0 gap-2">
              <Button variant="outline" size="sm" disabled={index <= 0} onClick={() => step(-1)}>
                {t(`${SECTION}.careerChartPrevious`)}
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={index >= bars.length - 1}
                onClick={() => step(1)}
              >
                {t(`${SECTION}.careerChartNext`)}
              </Button>
            </div>
          </div>
        </div>
        <Readout
          bar={selected}
          isFirst={index === 0}
          after={index === bars.length - 1 ? trailingMoves : []}
          language={language}
          onOpenTeam={onOpenTeam}
          t={t}
        />
      </div>
    </div>
  );
}
