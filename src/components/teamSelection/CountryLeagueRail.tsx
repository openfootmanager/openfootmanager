import { ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";

import { countryName } from "../../lib/countries";
import { CountryFlag } from "../ui";

interface RailLeague {
  id: string;
  name: string;
  /** The league's clubs. */
  clubCount: number;
}

interface CountryLeagueRailProps {
  countryCodes: readonly string[];
  selectedCountryCode: string | null;
  onSelectCountry: (code: string) => void;
  /** The selected country's domestic leagues, strongest first. */
  leagues: readonly RailLeague[];
  activeLeagueId: string | null;
  onSelectLeague: (id: string) => void;
}

const ROW_FOCUS =
  "focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus";

function LeagueList({
  leagues,
  activeLeagueId,
  onSelectLeague,
}: Pick<CountryLeagueRailProps, "leagues" | "activeLeagueId" | "onSelectLeague">) {
  const { t } = useTranslation();
  return (
    <ul className="flex flex-col gap-px">
      {leagues.map((league) => {
        const active = league.id === activeLeagueId;
        return (
          <li key={league.id}>
            <button
              type="button"
              aria-current={active ? "true" : undefined}
              onClick={() => onSelectLeague(league.id)}
              className={`flex min-h-9 w-full items-center justify-between gap-2 rounded-lg py-1 pl-8 pr-2.5 text-left text-body ${ROW_FOCUS} ${
                active
                  ? "bg-selected-tint font-semibold text-selected"
                  : "text-fg-muted hover:bg-hover"
              }`}
            >
              <span className="min-w-0 [overflow-wrap:anywhere]">{league.name}</span>
              <span className="tabular-nums" aria-hidden="true">
                {league.clubCount}
              </span>
              <span className="sr-only">{t("teamSelect.clubCount", { n: league.clubCount })}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

/**
 * The first two steps of choosing a club: a country, then one of its leagues. The picked country
 * (`aria-current`) opens to its leagues, each with its clubs counted; with no country to pick (a world whose
 * competitions place none) the leagues stand alone.
 */
export default function CountryLeagueRail({
  countryCodes,
  selectedCountryCode,
  onSelectCountry,
  leagues,
  activeLeagueId,
  onSelectLeague,
}: CountryLeagueRailProps) {
  const { t, i18n } = useTranslation();
  return (
    <nav
      aria-label={t("teamSelect.rail.label")}
      className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-line bg-panel shadow-card"
    >
      <div className="flex h-11 shrink-0 items-center justify-between gap-2 border-b border-line px-3.5">
        <h2 className="font-heading text-lead font-semibold uppercase tracking-label text-fg-muted">
          {t("teamSelect.rail.countries")}
        </h2>
        <span aria-hidden="true" className="text-caption text-fg-subtle">
          {countryCodes.length}
        </span>
      </div>
      <ul className="flex min-h-0 flex-col gap-px overflow-auto p-2">
        {countryCodes.length === 0 ? (
          <li>
            <LeagueList
              leagues={leagues}
              activeLeagueId={activeLeagueId}
              onSelectLeague={onSelectLeague}
            />
          </li>
        ) : null}
        {countryCodes.map((code) => {
          const open = code === selectedCountryCode;
          return (
            <li key={code}>
              <button
                type="button"
                aria-current={open ? "true" : undefined}
                onClick={() => onSelectCountry(code)}
                className={`flex min-h-9 w-full items-center gap-2 rounded-lg px-2.5 py-1 text-left text-body font-semibold text-fg hover:bg-hover ${ROW_FOCUS}`}
              >
                <CountryFlag
                  code={code}
                  locale={i18n.language}
                  decorative
                  className="h-3 w-[18px] shrink-0 rounded-sm"
                />
                <span className="min-w-0 flex-1 [overflow-wrap:anywhere]">
                  {countryName(code, i18n.language)}
                </span>
                <ChevronRight
                  aria-hidden="true"
                  className={`size-3.5 shrink-0 text-fg-subtle transition-transform motion-reduce:transition-none ${
                    open ? "rotate-90" : ""
                  }`}
                />
              </button>
              {open ? (
                <LeagueList
                  leagues={leagues}
                  activeLeagueId={activeLeagueId}
                  onSelectLeague={onSelectLeague}
                />
              ) : null}
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
