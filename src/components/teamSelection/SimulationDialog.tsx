import { AlertTriangle, Lock } from "lucide-react";
import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { countryName } from "../../lib/countries";
import { competitionDisplayName } from "../../lib/competitionName";
import { buildRegionLabel } from "../../lib/teamRegions";
import type { LeagueData, TeamData, WorldRegionData } from "../../store/gameStore";
import { Button, Checkbox, Dialog, DialogCloseButton, Select, Switch } from "../ui";
import {
  activeRegionIds,
  clubsInRegions,
  competitionState,
  enabledCompetitionIds,
  groupCompetitions,
  lockedRegions,
  type RegionLock,
  type ScopeDraft,
} from "./simulationScope";

interface SimulationDialogProps {
  regions: readonly WorldRegionData[];
  /** Every competition of the world, in the order to list them. */
  competitions: readonly LeagueData[];
  teams: readonly TeamData[];
  /** The chosen club's competitions: always simulated, so locked. None until a club is chosen. */
  yourCompetitions: readonly LeagueData[];
  /** The settings as they stand: the draft starts here, and Cancel leaves them so. */
  initial: ScopeDraft;
  onApply: (draft: ScopeDraft) => void;
  onCancel: () => void;
}

const SECTION_TITLE = "font-heading text-lead font-semibold uppercase tracking-label text-fg-muted";

function LockChip({ lock, id }: { lock: RegionLock; id: string }) {
  const { t } = useTranslation();
  return (
    <span
      id={id}
      className="inline-flex h-5 items-center gap-1 rounded-md bg-info-tint px-2 text-caption font-semibold text-info"
    >
      <Lock aria-hidden="true" className="size-3" />
      {lock === "home" ? t("teamSelect.homeBadge") : t("teamSelect.yourClubBadge")}
    </span>
  );
}

/**
 * What the world simulates (team selection A, the approved dialog): where the career starts, the
 * regions simulated and the competitions, in a dialog the player applies or cancels. The home
 * region and the regions the chosen club's competitions need are locked on; a competition whose
 * region is off is blocked and says which region it needs, with a button to add it. Everything is
 * a draft until Apply, so Cancel (the close button, Escape, the footer) leaves the settings as they
 * were.
 */
export default function SimulationDialog({
  regions,
  competitions,
  teams,
  yourCompetitions,
  initial,
  onApply,
  onCancel,
}: SimulationDialogProps) {
  const { t, i18n } = useTranslation();
  const titleId = useId();
  const subtitleId = useId();
  const idBase = useId();
  const [draft, setDraft] = useState<ScopeDraft>(initial);
  // The competition whose checkbox takes focus once "Add" has switched its region on: the button
  // that held focus is gone by then.
  const [focusCompetitionId, setFocusCompetitionId] = useState<string | null>(null);

  const locks = lockedRegions(draft.homeRegionId, yourCompetitions);
  const active = new Set(activeRegionIds(draft, locks));
  const yourIds = new Set(yourCompetitions.map((competition) => competition.id));
  const enabledCount = enabledCompetitionIds(competitions, draft, yourIds, active).length;
  const homeRegion = regions.find((region) => region.id === draft.homeRegionId);
  const regionLabel = (regionId: string) =>
    buildRegionLabel(t, regionId, regions.find((region) => region.id === regionId)?.name);
  const checkboxId = (competitionId: string) => `${idBase}-${competitionId}`;

  useEffect(() => {
    if (!focusCompetitionId) return;
    document.getElementById(checkboxId(focusCompetitionId))?.focus();
    setFocusCompetitionId(null);
  }, [focusCompetitionId]);

  function setHomeRegion(regionId: string) {
    const region = regions.find((candidate) => candidate.id === regionId);
    setDraft((current) => ({
      ...current,
      homeRegionId: regionId,
      homeCountryCode: region?.country_codes[0] ?? null,
    }));
  }

  function setRegionOn(regionId: string, on: boolean) {
    setDraft((current) => ({ ...current, regions: { ...current.regions, [regionId]: on } }));
  }

  function addMissingRegions(competition: LeagueData, regionIds: readonly string[]) {
    setDraft((current) => ({
      ...current,
      regions: { ...current.regions, ...Object.fromEntries(regionIds.map((id) => [id, true])) },
    }));
    setFocusCompetitionId(competition.id);
  }

  /** Where a competition is played: its country, else its region, else what kind of side plays it. */
  function whereOf(competition: LeagueData): string {
    if (competition.country_id) return countryName(competition.country_id, i18n.language);
    if (competition.region_id) return regionLabel(competition.region_id);
    return t(`teamSelect.kinds.${competition.kind}`, { defaultValue: competition.kind ?? "" });
  }

  return (
    <Dialog
      labelledBy={titleId}
      describedBy={subtitleId}
      onClose={onCancel}
      overlayClassName="bg-overlay p-4"
      className="flex max-h-[calc(100vh-2rem)] w-[56rem] max-w-[calc(100vw-2rem)] flex-col overflow-hidden rounded-2xl"
    >
      <div className="flex shrink-0 items-start gap-3 border-b border-line px-6 pb-3.5 pt-4">
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <h2
            id={titleId}
            className="font-heading text-title font-bold uppercase leading-tight tracking-label"
          >
            {t("teamSelect.scope.title")}
          </h2>
          <p id={subtitleId} className="text-body text-fg-muted">
            {t("teamSelect.scope.subtitle")}
          </p>
        </div>
        <DialogCloseButton onClose={onCancel} />
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 overflow-auto md:grid-cols-[21rem_minmax(0,1fr)] md:grid-rows-[minmax(0,1fr)] md:overflow-hidden">
        <div className="flex flex-col gap-5 border-b border-line p-5 md:overflow-auto md:border-b-0 md:border-r">
          <section className="flex flex-col gap-2.5">
            <h3 className={SECTION_TITLE}>{t("teamSelect.scope.sectionStart")}</h3>
            <div className="flex flex-col gap-1">
              <span className="text-caption text-fg-subtle">{t("teamSelect.homeRegion")}</span>
              <Select
                fullWidth
                value={draft.homeRegionId ?? ""}
                aria-label={t("teamSelect.homeRegion")}
                onChange={(event) => setHomeRegion(event.target.value)}
              >
                {regions.map((region) => (
                  <option key={region.id} value={region.id}>
                    {regionLabel(region.id)}
                  </option>
                ))}
              </Select>
            </div>
            <div className="flex flex-col gap-1">
              <span className="text-caption text-fg-subtle">{t("teamSelect.homeCountry")}</span>
              <Select
                fullWidth
                value={draft.homeCountryCode ?? ""}
                aria-label={t("teamSelect.homeCountry")}
                onChange={(event) =>
                  setDraft((current) => ({ ...current, homeCountryCode: event.target.value }))
                }
              >
                {(homeRegion?.country_codes ?? []).map((code) => (
                  <option key={code} value={code}>
                    {countryName(code, i18n.language)}
                  </option>
                ))}
              </Select>
            </div>
          </section>

          <section className="flex flex-col gap-2">
            <h3 className={SECTION_TITLE}>{t("teamSelect.scope.sectionRegions")}</h3>
            <ul className="flex flex-col gap-1.5">
              {regions.map((region) => {
                const lock = locks.get(region.id);
                const nameId = `${idBase}-region-${region.id}`;
                return (
                  <li
                    key={region.id}
                    className="flex min-h-11 items-center gap-2.5 rounded-lg border border-line bg-panel px-3 py-1.5"
                  >
                    <Switch
                      labelledBy={nameId}
                      describedBy={lock ? `${nameId}-lock` : undefined}
                      checked={active.has(region.id)}
                      disabled={lock !== undefined}
                      onChange={(on) => setRegionOn(region.id, on)}
                    />
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span id={nameId} className="font-semibold">
                        {regionLabel(region.id)}
                      </span>
                      <span className="text-caption text-fg-subtle [overflow-wrap:anywhere]">
                        {t("teamSelect.scope.regionMeta", {
                          countries: region.country_codes
                            .map((code) => countryName(code, i18n.language))
                            .join(", "),
                          clubs: clubsInRegions(regions, new Set([region.id]), teams),
                        })}
                      </span>
                    </span>
                    {lock ? <LockChip lock={lock} id={`${nameId}-lock`} /> : null}
                  </li>
                );
              })}
            </ul>
          </section>
        </div>

        <div className="flex min-h-0 flex-col">
          <h3 className={`${SECTION_TITLE} shrink-0 px-5 pb-2 pt-4`}>
            {t("teamSelect.scope.sectionCompetitions")}
          </h3>
          <div className="min-h-0 flex-1 overflow-auto border-t border-line">
            {groupCompetitions(competitions).map((group) => {
              const states = group.competitions.map((competition) =>
                competitionState(competition, draft, yourIds, active),
              );
              const groupId = `${idBase}-group-${group.scope}`;
              const groupName = t(`teamSelect.scopes.${group.scope}`, {
                defaultValue: group.scope,
              });
              const counts = {
                on: states.filter((state) => state.on).length,
                total: states.length,
              };
              return (
                <section key={group.scope} aria-labelledby={groupId}>
                  <div className="flex h-8 items-center justify-between border-y border-line bg-sunken px-3 text-caption font-bold uppercase tracking-label text-fg-muted">
                    <h4 id={groupId}>{groupName}</h4>
                    <span
                      aria-hidden="true"
                      className="font-semibold normal-case tracking-normal tabular-nums"
                    >
                      {t("teamSelect.scope.groupCount", counts)}
                    </span>
                    <span className="sr-only">{t("teamSelect.scope.groupSimulated", counts)}</span>
                  </div>
                  <ul>
                    {group.competitions.map((competition, index) => {
                      const state = states[index];
                      const name = competitionDisplayName(competition, t);
                      const missing = state.missingRegionIds.map(regionLabel).join(", ");
                      const nameId = `${checkboxId(competition.id)}-name`;
                      const reasonId = `${checkboxId(competition.id)}-reason`;
                      return (
                        <li
                          key={competition.id}
                          className="flex min-h-10 items-center gap-2.5 border-b border-line px-3 py-1"
                        >
                          <Checkbox
                            id={checkboxId(competition.id)}
                            checked={state.on}
                            disabled={state.yours || state.blocked}
                            aria-label={t("teamSelect.scope.simulate", { name })}
                            aria-describedby={state.yours || state.blocked ? reasonId : undefined}
                            onChange={() =>
                              setDraft((current) => ({
                                ...current,
                                competitions: {
                                  ...current.competitions,
                                  [competition.id]: current.competitions[competition.id] !== true,
                                },
                              }))
                            }
                          />
                          <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                            <span className="flex flex-wrap items-center gap-2">
                              {/* The name toggles the box too, as a label does. */}
                              <label
                                id={nameId}
                                htmlFor={checkboxId(competition.id)}
                                className="font-semibold"
                              >
                                {name}
                              </label>
                              <span className="inline-flex h-5 items-center rounded-md border border-line bg-sunken px-2 text-caption font-semibold text-fg-muted">
                                {whereOf(competition)}
                              </span>
                              {state.yours ? <LockChip lock="yourClub" id={reasonId} /> : null}
                            </span>
                            {state.blocked ? (
                              <span className="flex flex-wrap items-center gap-1.5 text-caption text-warning">
                                <AlertTriangle aria-hidden="true" className="size-3.5" />
                                <span id={reasonId}>
                                  {t("teamSelect.scope.needs", { regions: missing })}
                                </span>
                                <button
                                  type="button"
                                  aria-describedby={nameId}
                                  onClick={() =>
                                    addMissingRegions(competition, state.missingRegionIds)
                                  }
                                  className="rounded-sm font-semibold text-selected underline focus:outline-none focus-visible:ring-2 focus-visible:ring-focus"
                                >
                                  {t("teamSelect.scope.add", { regions: missing })}
                                </button>
                              </span>
                            ) : null}
                          </span>
                        </li>
                      );
                    })}
                  </ul>
                </section>
              );
            })}
          </div>
        </div>
      </div>

      <div className="flex shrink-0 flex-wrap items-center gap-3 border-t border-line bg-sunken px-6 py-3">
        <p className="flex min-w-0 flex-col">
          <span className="font-semibold tabular-nums">
            {t("teamSelect.scope.summary", { regions: active.size, competitions: enabledCount })}
          </span>
          <span className="text-caption text-fg-subtle">
            {t("teamSelect.scope.clubs", { clubs: clubsInRegions(regions, active, teams) })}
          </span>
        </p>
        <div className="ml-auto flex items-center gap-2.5">
          <Button variant="ghost" onClick={onCancel}>
            {t("common.cancel")}
          </Button>
          <Button onClick={() => onApply(draft)}>{t("teamSelect.scope.apply")}</Button>
        </div>
      </div>
    </Dialog>
  );
}
