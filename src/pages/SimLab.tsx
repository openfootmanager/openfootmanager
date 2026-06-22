import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

type PlayStyleDto =
  | "balanced"
  | "attacking"
  | "defensive"
  | "possession"
  | "counter"
  | "high_press";

interface SimBatchConfig {
  games: number;
  seed: number | null;
  home_style: PlayStyleDto;
  away_style: PlayStyleDto;
  home_formation: string;
  away_formation: string;
  home_rating: number;
  away_rating: number;
  home_advantage: number | null;
  shot_accuracy_base: number | null;
  goal_conversion_base: number | null;
  foul_probability: number | null;
  yellow_card_probability: number | null;
  red_card_probability: number | null;
  penalty_probability: number | null;
  injury_probability: number | null;
}

interface SimBatchResults {
  games: number;
  home_wins: number;
  draws: number;
  away_wins: number;
  home_win_pct: number;
  draw_pct: number;
  away_win_pct: number;
  goals_per_game: number;
  home_goals_per_game: number;
  away_goals_per_game: number;
  clean_sheet_home_pct: number;
  clean_sheet_away_pct: number;
  btts_pct: number;
  shots_per_game: number;
  shots_on_target_pct: number;
  goal_conversion_pct: number;
  xg_proxy_per_game: number;
  yellow_cards_per_game: number;
  red_cards_per_game: number;
  fouls_per_game: number;
  penalties_per_game: number;
  penalty_conversion_pct: number;
  injuries_per_game: number;
  corners_per_game: number;
  free_kicks_per_game: number;
  home_possession_avg: number;
  away_possession_avg: number;
  passes_per_game: number;
  goals_per_game_hist: number[];
  scoreline_heatmap: number[][];
  goals_by_bucket: number[];
  total_time_secs: number;
  games_per_sec: number;
}

// ── Constants ─────────────────────────────────────────────────────────────────

const STYLES: { value: PlayStyleDto; label: string }[] = [
  { value: "balanced", label: "Balanced" },
  { value: "attacking", label: "Attacking" },
  { value: "defensive", label: "Defensive" },
  { value: "possession", label: "Possession" },
  { value: "counter", label: "Counter" },
  { value: "high_press", label: "High Press" },
];

const FORMATIONS = [
  "4-4-2",
  "4-3-3",
  "4-5-1",
  "3-5-2",
  "5-3-2",
  "4-2-3-1",
  "3-4-3",
];

const BUCKET_LABELS = [
  "1–15",
  "16–30",
  "31–45",
  "46–60",
  "61–75",
  "76–90",
  "90+",
];

const TABS = ["Overview", "Shooting", "Discipline", "Heatmap", "Timeline", "Benchmark"] as const;
type Tab = (typeof TABS)[number];

const REAL_TARGETS: [string, string, number, number][] = [
  ["Goals/game", "2.3–3.0", 2.3, 3.0],
  ["Shots/game", "18–32", 18, 32],
  ["Shot accuracy %", "32–45%", 32, 45],
  ["Goal conversion %", "20–40%", 20, 40],
  ["Yellow cards/game", "2.0–4.0", 2.0, 4.0],
  ["Red cards/game", "0.05–0.15", 0.05, 0.15],
  ["Fouls/game", "18–28", 18, 28],
  ["Corners/game", "8–14", 8, 14],
  ["Home win %", "40–52%", 40, 52],
  ["Clean sheet %", "22–35%", 22, 35],
  ["Penalties/game", "0.20–0.50", 0.2, 0.5],
  ["Pen. conversion %", "65–85%", 65, 85],
  ["BTTS %", "50–55%", 50, 55],
];

// ── Default config ─────────────────────────────────────────────────────────────

const defaultConfig = (): SimBatchConfig => ({
  games: 1000,
  seed: null,
  home_style: "balanced",
  away_style: "balanced",
  home_formation: "4-4-2",
  away_formation: "4-4-2",
  home_rating: 70,
  away_rating: 70,
  home_advantage: null,
  shot_accuracy_base: null,
  goal_conversion_base: null,
  foul_probability: null,
  yellow_card_probability: null,
  red_card_probability: null,
  penalty_probability: null,
  injury_probability: null,
});

// ── Main page ─────────────────────────────────────────────────────────────────

export default function SimLab() {
  const [cfg, setCfg] = useState<SimBatchConfig>(defaultConfig);
  const [results, setResults] = useState<SimBatchResults | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("Overview");

  const run = useCallback(async () => {
    setRunning(true);
    setError(null);
    try {
      const res = await invoke<SimBatchResults>("run_sim_batch", { config: cfg });
      setResults(res);
      setActiveTab("Overview");
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }, [cfg]);

  const update = <K extends keyof SimBatchConfig>(key: K, value: SimBatchConfig[K]) =>
    setCfg((prev) => ({ ...prev, [key]: value }));

  return (
    <div className="min-h-screen bg-navy-900 text-slate-100 flex flex-col" style={{ fontFamily: "system-ui, sans-serif" }}>
      {/* Header */}
      <div className="border-b border-navy-700 px-6 py-3 flex items-center justify-between bg-navy-900">
        <div>
          <h1 className="text-lg font-bold text-blue-400">Simulation Lab</h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Batch match simulation &amp; engine analysis tool
          </p>
        </div>
        <button
          onClick={() => { void run(); }}
          disabled={running}
          className="px-5 py-2 rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed font-semibold text-sm transition-colors"
        >
          {running ? "Simulating…" : `Run ${cfg.games.toLocaleString()} games`}
        </button>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Config sidebar */}
        <aside className="w-72 flex-shrink-0 border-r border-navy-700 bg-navy-900 overflow-y-auto p-4 space-y-5">
          <Section title="Teams">
            <TeamConfig side="Home" cfg={cfg} onChange={update} prefix="home" />
            <div className="my-3 border-t border-navy-700" />
            <TeamConfig side="Away" cfg={cfg} onChange={update} prefix="away" />
          </Section>

          <Section title="Simulation">
            <Label htmlFor="games-input">Games (1 – 100,000)</Label>
            <input
              id="games-input"
              type="number"
              min={1}
              max={100000}
              step={1}
              value={cfg.games}
              onChange={(e) => {
                const v = Math.max(1, Math.min(100000, Math.floor(Number(e.target.value))));
                if (!isNaN(v)) update("games", v);
              }}
              className={inputCls}
            />
            {cfg.games > 10000 && (
              <p className="text-xs text-amber-400 mt-1">Large runs may take several seconds.</p>
            )}
            <Label htmlFor="seed-input">Seed (blank = random)</Label>
            <input
              id="seed-input"
              type="number"
              min={0}
              step={1}
              placeholder="e.g. 42"
              value={cfg.seed ?? ""}
              onChange={(e) =>
                update("seed", e.target.value === "" ? null : Math.max(0, Math.floor(Number(e.target.value))))
              }
              className={inputCls}
            />
          </Section>

          <Section title="Engine Parameters">
            <ConfigSlider
              label="Home advantage"
              value={cfg.home_advantage ?? 1.08}
              min={1.0}
              max={1.2}
              step={0.01}
              defaultVal={1.08}
              onChange={(v) => update("home_advantage", v)}
            />
            <ConfigSlider
              label="Shot accuracy base"
              value={cfg.shot_accuracy_base ?? 0.45}
              min={0.2}
              max={0.8}
              step={0.01}
              defaultVal={0.45}
              onChange={(v) => update("shot_accuracy_base", v)}
            />
            <ConfigSlider
              label="Goal conversion base"
              value={cfg.goal_conversion_base ?? 0.3}
              min={0.1}
              max={0.6}
              step={0.01}
              defaultVal={0.3}
              onChange={(v) => update("goal_conversion_base", v)}
            />
            <ConfigSlider
              label="Foul probability"
              value={cfg.foul_probability ?? 0.12}
              min={0.05}
              max={0.4}
              step={0.01}
              defaultVal={0.12}
              onChange={(v) => update("foul_probability", v)}
            />
            <ConfigSlider
              label="Yellow card prob."
              value={cfg.yellow_card_probability ?? 0.3}
              min={0.1}
              max={0.6}
              step={0.01}
              defaultVal={0.3}
              onChange={(v) => update("yellow_card_probability", v)}
            />
            <ConfigSlider
              label="Red card prob."
              value={cfg.red_card_probability ?? 0.04}
              min={0.01}
              max={0.15}
              step={0.005}
              defaultVal={0.04}
              onChange={(v) => update("red_card_probability", v)}
            />
            <ConfigSlider
              label="Penalty probability"
              value={cfg.penalty_probability ?? 0.08}
              min={0.02}
              max={0.25}
              step={0.01}
              defaultVal={0.08}
              onChange={(v) => update("penalty_probability", v)}
            />
            <ConfigSlider
              label="Injury probability"
              value={cfg.injury_probability ?? 0.03}
              min={0.0}
              max={0.15}
              step={0.005}
              defaultVal={0.03}
              onChange={(v) => update("injury_probability", v)}
            />
            <button
              onClick={() => setCfg(defaultConfig())}
              className="w-full mt-2 text-xs text-slate-500 hover:text-slate-300 underline"
            >
              Reset to defaults
            </button>
          </Section>
        </aside>

        {/* Results area */}
        <main className="flex-1 overflow-y-auto p-6">
          {error && (
            <div className="mb-4 p-3 bg-red-900/40 border border-red-700 rounded-lg text-red-300 text-sm">
              {error}
            </div>
          )}

          {!results && !running && (
            <EmptyState />
          )}

          {running && (
            <div className="flex items-center gap-3 text-slate-400">
              <div className="w-5 h-5 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
              Simulating {cfg.games.toLocaleString()} games…
            </div>
          )}

          {results && !running && (
            <>
              {/* Tab bar */}
              <div className="flex gap-1 mb-6 border-b border-navy-700">
                {TABS.map((t) => (
                  <button
                    key={t}
                    onClick={() => setActiveTab(t)}
                    className={`px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px ${
                      activeTab === t
                        ? "border-blue-400 text-blue-400"
                        : "border-transparent text-slate-500 hover:text-slate-300"
                    }`}
                  >
                    {t}
                  </button>
                ))}
              </div>

              {activeTab === "Overview" && <OverviewTab r={results} />}
              {activeTab === "Shooting" && <ShootingTab r={results} />}
              {activeTab === "Discipline" && <DisciplineTab r={results} />}
              {activeTab === "Heatmap" && <HeatmapTab r={results} />}
              {activeTab === "Timeline" && <TimelineTab r={results} />}
              {activeTab === "Benchmark" && <BenchmarkTab r={results} />}
            </>
          )}
        </main>
      </div>
    </div>
  );
}

// ── Config components ──────────────────────────────────────────────────────────

function TeamConfig({
  side,
  cfg,
  onChange,
  prefix,
}: {
  side: string;
  cfg: SimBatchConfig;
  onChange: <K extends keyof SimBatchConfig>(k: K, v: SimBatchConfig[K]) => void;
  prefix: "home" | "away";
}) {
  return (
    <div>
      <p className="text-xs font-semibold text-blue-400 mb-2">{side}</p>
      <Label htmlFor={`${prefix}-style`}>Style</Label>
      <select
        id={`${prefix}-style`}
        value={cfg[`${prefix}_style`]}
        onChange={(e) => onChange(`${prefix}_style`, e.target.value as PlayStyleDto)}
        className={inputCls}
      >
        {STYLES.map((s) => (
          <option key={s.value} value={s.value}>
            {s.label}
          </option>
        ))}
      </select>
      <Label htmlFor={`${prefix}-formation`}>Formation</Label>
      <select
        id={`${prefix}-formation`}
        value={cfg[`${prefix}_formation`]}
        onChange={(e) => onChange(`${prefix}_formation`, e.target.value)}
        className={inputCls}
      >
        {FORMATIONS.map((f) => (
          <option key={f} value={f}>
            {f}
          </option>
        ))}
      </select>
      <Label htmlFor={`${prefix}-rating`}>Overall Rating: {cfg[`${prefix}_rating`]}</Label>
      <input
        id={`${prefix}-rating`}
        type="range"
        min={40}
        max={95}
        value={cfg[`${prefix}_rating`]}
        onChange={(e) => onChange(`${prefix}_rating`, Number(e.target.value))}
        className="w-full accent-blue-400"
      />
    </div>
  );
}

function ConfigSlider({
  label,
  value,
  min,
  max,
  step,
  defaultVal,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  defaultVal: number;
  onChange: (v: number) => void;
}) {
  const isModified = Math.abs(value - defaultVal) > step / 2;
  return (
    <div className="mb-3">
      <div className="flex justify-between items-center mb-1">
        <Label>{label}</Label>
        <span className={`text-xs font-mono ${isModified ? "text-yellow-400" : "text-slate-400"}`}>
          {value.toFixed(3).replace(/\.?0+$/, "")}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-blue-400"
      />
    </div>
  );
}

// ── Tab content ───────────────────────────────────────────────────────────────

function OverviewTab({ r }: { r: SimBatchResults }) {
  const getVal = (label: string): number => {
    const map: Record<string, number> = {
      "Goals/game": r.goals_per_game,
      "Shots/game": r.shots_per_game,
      "Shot accuracy %": r.shots_on_target_pct,
      "Goal conversion %": r.goal_conversion_pct,
      "Yellow cards/game": r.yellow_cards_per_game,
      "Red cards/game": r.red_cards_per_game,
      "Fouls/game": r.fouls_per_game,
      "Corners/game": r.corners_per_game,
      "Home win %": r.home_win_pct,
      "Clean sheet %": r.clean_sheet_home_pct,
      "Penalties/game": r.penalties_per_game,
      "Pen. conversion %": r.penalty_conversion_pct,
      "BTTS %": r.btts_pct,
    };
    return map[label] ?? 0;
  };

  return (
    <div className="space-y-6">
      {/* Outcome bars */}
      <Card title="Match Outcomes">
        <div className="space-y-3">
          <OutcomeBar label="Home Win" value={r.home_win_pct} count={r.home_wins} color="#4f8ef7" />
          <OutcomeBar label="Draw" value={r.draw_pct} count={r.draws} color="#94a3b8" />
          <OutcomeBar label="Away Win" value={r.away_win_pct} count={r.away_wins} color="#f472b6" />
        </div>
      </Card>

      {/* Key numbers */}
      <div className="grid grid-cols-3 gap-4">
        <StatCard label="Goals/game" value={r.goals_per_game.toFixed(2)} sub={`H: ${r.home_goals_per_game.toFixed(2)} — A: ${r.away_goals_per_game.toFixed(2)}`} />
        <StatCard label="Clean sheets (H)" value={`${r.clean_sheet_home_pct.toFixed(1)}%`} sub="Home team kept" />
        <StatCard label="BTTS" value={`${r.btts_pct.toFixed(1)}%`} sub="Both scored" />
      </div>

      {/* Goals histogram */}
      <Card title="Goals per Game Distribution">
        <Histogram
          data={r.goals_per_game_hist}
          labels={["0","1","2","3","4","5","6","7","8","9+"]}
          color="#4f8ef7"
        />
      </Card>

      {/* Real-football benchmark table */}
      <Card title="Real-Football Benchmark Comparison">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-xs text-slate-500 uppercase border-b border-navy-700">
              <th className="py-2 text-left">Metric</th>
              <th className="py-2 text-right">Simulated</th>
              <th className="py-2 text-right">Target</th>
              <th className="py-2 text-right">Status</th>
            </tr>
          </thead>
          <tbody>
            {REAL_TARGETS.map(([label, target, lo, hi]) => {
              const v = getVal(label);
              const ok = v >= lo && v <= hi;
              return (
                <tr key={label} className="border-b border-navy-800">
                  <td className="py-1.5 text-slate-400">{label}</td>
                  <td className="py-1.5 text-right font-mono font-semibold">{v.toFixed(2)}</td>
                  <td className="py-1.5 text-right text-slate-500 text-xs">{target}</td>
                  <td className="py-1.5 text-right">
                    <span className={`text-xs font-semibold ${ok ? "text-green-400" : "text-red-400"}`}>
                      {ok ? "✓" : "✗"}
                    </span>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </Card>
    </div>
  );
}

function ShootingTab({ r }: { r: SimBatchResults }) {
  const xg = r.xg_proxy_per_game;
  const diff = r.goals_per_game - xg;
  return (
    <div className="space-y-6">
      <div className="grid grid-cols-3 gap-4">
        <StatCard label="Shots/game" value={r.shots_per_game.toFixed(1)} sub="Total both teams" />
        <StatCard label="On target %" value={`${r.shots_on_target_pct.toFixed(1)}%`} sub="Target: 32–45%" />
        <StatCard label="Conversion %" value={`${r.goal_conversion_pct.toFixed(1)}%`} sub="Target: 20–40%" />
      </div>

      <Card title="Shooting Funnel">
        <div className="space-y-4">
          <FunnelRow label="Total shots" value={r.shots_per_game} max={r.shots_per_game} color="#64748b" />
          <FunnelRow label="On target" value={r.shots_per_game * r.shots_on_target_pct / 100} max={r.shots_per_game} color="#4f8ef7" />
          <FunnelRow label="Goals" value={r.goals_per_game} max={r.shots_per_game} color="#34d399" />
        </div>
      </Card>

      <Card title="Expected Goals (Proxy)">
        <p className="text-xs text-slate-500 mb-4">
          xG proxy = shots on target/game × goal_conversion_base. Not per-shot (engine doesn't emit shot quality in static path).
        </p>
        <div className="grid grid-cols-3 gap-4">
          <StatCard label="xG/game" value={xg.toFixed(2)} sub="Proxy estimate" />
          <StatCard label="Goals/game" value={r.goals_per_game.toFixed(2)} sub="Actual" />
          <StatCard
            label="Goals vs xG"
            value={`${diff >= 0 ? "+" : ""}${diff.toFixed(2)}`}
            sub={diff >= 0 ? "Overperforming" : "Underperforming"}
            valueClass={diff >= 0 ? "text-green-400" : "text-yellow-400"}
          />
        </div>
      </Card>

      <Card title="Set Piece Shooting">
        <div className="grid grid-cols-3 gap-4">
          <StatCard label="Penalties/game" value={r.penalties_per_game.toFixed(2)} sub="Target: 0.20–0.50" />
          <StatCard label="Pen. conversion" value={`${r.penalty_conversion_pct.toFixed(1)}%`} sub="Target: 65–85%" />
          <StatCard label="Corners/game" value={r.corners_per_game.toFixed(1)} sub="Target: 8–14" />
        </div>
      </Card>
    </div>
  );
}

function DisciplineTab({ r }: { r: SimBatchResults }) {
  return (
    <div className="space-y-6">
      <div className="grid grid-cols-3 gap-4">
        <StatCard label="Fouls/game" value={r.fouls_per_game.toFixed(1)} sub="Target: 18–28" valueClass={inRange(r.fouls_per_game, 18, 28)} />
        <StatCard label="Yellow cards/game" value={r.yellow_cards_per_game.toFixed(2)} sub="Target: 2.0–4.0" valueClass={inRange(r.yellow_cards_per_game, 2, 4)} />
        <StatCard label="Red cards/game" value={r.red_cards_per_game.toFixed(3)} sub="Target: 0.05–0.15" valueClass={inRange(r.red_cards_per_game, 0.05, 0.15)} />
      </div>
      <div className="grid grid-cols-2 gap-4">
        <StatCard label="Injuries/game" value={r.injuries_per_game.toFixed(2)} sub="" />
        <StatCard label="Free kicks/game" value={r.free_kicks_per_game.toFixed(1)} sub="" />
      </div>

      <Card title="Discipline Notes">
        <p className="text-sm text-slate-400 leading-relaxed">
          The static simulation engine only triggers fouls in the midfield and attacking-third zones.
          Box fouls (leading to penalties) are not generated via this path — penalties will show as 0
          regardless of the <code className="bg-navy-800 px-1 rounded">penalty_probability</code> config.
          This is a known limitation of the static engine path. The live match engine handles this correctly.
        </p>
        <p className="text-sm text-slate-400 mt-2 leading-relaxed">
          If fouls per game is below 18, consider increasing <code className="bg-navy-800 px-1 rounded">foul_probability</code> or
          reviewing how often tackle events occur in the zone resolution logic.
        </p>
      </Card>
    </div>
  );
}

function HeatmapTab({ r }: { r: SimBatchResults }) {
  const max = Math.max(...r.scoreline_heatmap.flat());
  return (
    <div className="space-y-6">
      <Card title="Scoreline Heatmap — Home Goals (rows) × Away Goals (cols)">
        <p className="text-xs text-slate-500 mb-4">Goals capped at 5+. Cell = % of games.</p>
        <div className="overflow-x-auto">
          <table className="border-collapse">
            <thead>
              <tr>
                <th className="w-10 h-10" />
                {[0,1,2,3,4,5].map((ag) => (
                  <th key={ag} className="w-14 h-10 text-xs text-slate-500 text-center font-normal">
                    {ag === 5 ? "5+" : ag}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {r.scoreline_heatmap.map((row, hg) => (
                <tr key={hg}>
                  <td className="w-10 text-xs text-slate-500 text-right pr-2">
                    {hg === 5 ? "5+" : hg}
                  </td>
                  {row.map((frac, ag) => {
                    const opacity = max > 0 ? frac / max : 0;
                    const pct = (frac * 100).toFixed(1);
                    return (
                      <td
                        key={ag}
                        title={`${hg}-${ag}: ${pct}%`}
                        className="w-14 h-10 text-center text-xs font-semibold rounded"
                        style={{
                          backgroundColor: `rgba(79,142,247,${Math.max(0.04, opacity * 0.9)})`,
                          color: opacity > 0.55 ? "#fff" : "#64748b",
                        }}
                      >
                        {frac > 0 ? `${pct}%` : ""}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}

function TimelineTab({ r }: { r: SimBatchResults }) {
  const max = Math.max(...r.goals_by_bucket, 0.001);
  return (
    <div className="space-y-6">
      <Card title="Scoring Timeline — Goals by 15-minute Interval">
        <p className="text-xs text-slate-500 mb-4">
          Fraction of all goals scored in each 15-minute window.
        </p>
        <div className="space-y-3">
          {r.goals_by_bucket.map((frac, i) => (
            <div key={i} className="flex items-center gap-3">
              <span className="text-xs text-slate-500 w-12 text-right">{BUCKET_LABELS[i]}</span>
              <div className="flex-1 h-5 bg-navy-800 rounded overflow-hidden">
                <div
                  className="h-full rounded bg-blue-500 transition-all"
                  style={{ width: `${(frac / max) * 100}%` }}
                />
              </div>
              <span className="text-xs text-slate-400 w-12 text-right">
                {(frac * 100).toFixed(1)}%
              </span>
            </div>
          ))}
        </div>
        <p className="text-xs text-slate-500 mt-4">
          A fairly even distribution across all periods is expected. Goals should increase slightly
          in the 76–90 window as teams push for results. Check for anomalies in the extra time bucket.
        </p>
      </Card>

      <Card title="Possession">
        <div className="mb-2 text-xs text-slate-500">Average possession split across {r.games.toLocaleString()} games</div>
        <div className="flex h-7 rounded overflow-hidden gap-px">
          <div
            className="flex items-center justify-center text-xs font-bold text-white"
            style={{ width: `${r.home_possession_avg}%`, background: "#4f8ef7" }}
          >
            {r.home_possession_avg.toFixed(1)}%
          </div>
          <div
            className="flex items-center justify-center text-xs font-bold text-white"
            style={{ width: `${r.away_possession_avg}%`, background: "#f472b6" }}
          >
            {r.away_possession_avg.toFixed(1)}%
          </div>
        </div>
        <div className="mt-3 grid grid-cols-2 gap-4">
          <StatCard label="Passes completed/game" value={r.passes_per_game.toFixed(0)} sub="" />
          <StatCard label="Corners/game" value={r.corners_per_game.toFixed(1)} sub="" />
        </div>
      </Card>
    </div>
  );
}

function BenchmarkTab({ r }: { r: SimBatchResults }) {
  return (
    <div className="space-y-6">
      <div className="grid grid-cols-3 gap-4">
        <StatCard label="Games simulated" value={r.games.toLocaleString()} sub="" />
        <StatCard label="Total time" value={`${r.total_time_secs.toFixed(2)}s`} sub="" />
        <StatCard label="Throughput" value={`${Math.round(r.games_per_sec).toLocaleString()}`} sub="games / second" />
      </div>

      <Card title="Interpretation">
        <p className="text-sm text-slate-400 leading-relaxed">
          This is single-threaded throughput for the static simulation engine (
          <code className="bg-navy-800 px-1 rounded">engine::simulate_with_rng</code>).
          For deeper analysis, use the <code className="bg-navy-800 px-1 rounded">ofm-sim-bench --bench</code> CLI
          which measures per-game latency distribution (p50/p95/p99).
        </p>
        <p className="text-sm text-slate-400 mt-2 leading-relaxed">
          At {Math.round(r.games_per_sec).toLocaleString()} games/sec you can run 100,000 games in approximately{" "}
          {(100000 / r.games_per_sec).toFixed(1)} seconds — useful for large parameter sweeps.
        </p>
      </Card>
    </div>
  );
}

// ── Shared UI components ──────────────────────────────────────────────────────

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <p className="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-3">{title}</p>
      {children}
    </div>
  );
}

function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="bg-navy-900 border border-navy-700 rounded-xl p-5">
      <h3 className="text-xs font-bold uppercase tracking-widest text-slate-500 mb-4">{title}</h3>
      {children}
    </div>
  );
}

function Label({ children, htmlFor }: { children: React.ReactNode; htmlFor?: string }) {
  return <label htmlFor={htmlFor} className="text-xs text-slate-500 mb-1 mt-2 block">{children}</label>;
}

function StatCard({
  label,
  value,
  sub,
  valueClass,
}: {
  label: string;
  value: string;
  sub: string;
  valueClass?: string;
}) {
  return (
    <div className="bg-navy-800 rounded-lg p-4">
      <p className="text-xs text-slate-500 mb-1">{label}</p>
      <p className={`text-2xl font-bold ${valueClass ?? "text-slate-100"}`}>{value}</p>
      {sub && <p className="text-xs text-slate-500 mt-1">{sub}</p>}
    </div>
  );
}

function OutcomeBar({
  label,
  value,
  count,
  color,
}: {
  label: string;
  value: number;
  count: number;
  color: string;
}) {
  return (
    <div className="flex items-center gap-3">
      <span className="w-20 text-sm text-slate-400">{label}</span>
      <div className="flex-1 h-5 bg-navy-800 rounded overflow-hidden">
        <div
          className="h-full rounded transition-all"
          style={{ width: `${value}%`, backgroundColor: color }}
        />
      </div>
      <span className="text-sm font-semibold w-14 text-right">{value.toFixed(1)}%</span>
      <span className="text-xs text-slate-500 w-12 text-right">{count.toLocaleString()}</span>
    </div>
  );
}

function Histogram({
  data,
  labels,
  color,
}: {
  data: number[];
  labels: string[];
  color: string;
}) {
  const max = Math.max(...data, 0.001);
  return (
    <div className="flex items-end gap-1.5" style={{ height: 100 }}>
      {data.map((v, i) => (
        <div key={i} className="flex-1 flex flex-col items-center gap-1">
          <div
            className="w-full rounded-t transition-all"
            title={`${labels[i]}: ${(v * 100).toFixed(1)}%`}
            style={{
              height: `${(v / max) * 80}px`,
              backgroundColor: color,
              minHeight: v > 0 ? 2 : 0,
            }}
          />
          <span className="text-xs text-slate-500">{labels[i]}</span>
        </div>
      ))}
    </div>
  );
}

function FunnelRow({
  label,
  value,
  max,
  color,
}: {
  label: string;
  value: number;
  max: number;
  color: string;
}) {
  return (
    <div className="flex items-center gap-3">
      <span className="w-28 text-sm text-slate-400">{label}</span>
      <div className="flex-1 h-5 bg-navy-800 rounded overflow-hidden">
        <div
          className="h-full rounded"
          style={{ width: `${(value / max) * 100}%`, backgroundColor: color }}
        />
      </div>
      <span className="text-sm font-semibold w-10 text-right">{value.toFixed(1)}</span>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center h-64 text-center text-slate-500">
      <div className="text-4xl mb-3">⚽</div>
      <p className="font-semibold">Configure and run a simulation</p>
      <p className="text-sm mt-1">Adjust the parameters on the left, then press Run.</p>
    </div>
  );
}

// ── Utilities ─────────────────────────────────────────────────────────────────

const inputCls =
  "w-full bg-navy-800 border border-navy-600 rounded-md px-2.5 py-1.5 text-sm text-slate-200 focus:outline-none focus:border-blue-500";

function inRange(value: number, lo: number, hi: number): string {
  return value >= lo && value <= hi ? "text-green-400" : "text-red-400";
}
