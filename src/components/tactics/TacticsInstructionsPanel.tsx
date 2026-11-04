import type { JSX, ReactNode } from "react";
import { ChevronDown, ChevronsLeft, ChevronsRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TacticsPhaseSettings } from "../../store/types";
import { Select } from "../ui";
import { PHASE_SECTIONS } from "./PhaseBlueprintPanel";

/** The panel's sections, in the order a match is played, then the set pieces. */
export type InstructionsSection = "withBall" | "transitions" | "withoutBall" | "setPieces";

const SECTION_ORDER: readonly InstructionsSection[] = [
  "withBall",
  "transitions",
  "withoutBall",
  "setPieces",
];

const SECTION_TITLE_KEYS: Record<InstructionsSection, string> = {
  withBall: "tactics.phaseLabels.withBall",
  transitions: "tactics.phaseLabels.transitions",
  withoutBall: "tactics.phaseLabels.withoutBall",
  setPieces: "tactics.setPiecesSection",
};

const PANEL_ID = "tactics-instructions";
/** The panel's heading, which names it, and names the drawer it opens in. */
export const INSTRUCTIONS_TITLE_ID = `${PANEL_ID}-title`;

interface TacticsInstructionsPanelProps {
  /** The panel is open; folded, it is a rail of the sections. */
  open: boolean;
  /** The sections shown open in the accordion. */
  openSections: ReadonlySet<InstructionsSection>;
  onOpenChange: (open: boolean) => void;
  onToggleSection: (section: InstructionsSection) => void;
  /** Opens the panel with `section` open, from the rail. */
  onOpenAt: (section: InstructionsSection) => void;
  tacticsPhase?: TacticsPhaseSettings;
  onTacticsPhaseChange: (patch: Partial<TacticsPhaseSettings>) => void;
  /** The set-piece takers' section body. */
  setPieces: ReactNode;
}

function PhaseFields({
  section,
  tacticsPhase,
  onTacticsPhaseChange,
}: {
  section: Exclude<InstructionsSection, "setPieces">;
  tacticsPhase?: TacticsPhaseSettings;
  onTacticsPhaseChange: (patch: Partial<TacticsPhaseSettings>) => void;
}): JSX.Element {
  const { t } = useTranslation();
  const fields = PHASE_SECTIONS.find(([key]) => key === section)?.[1] ?? [];
  return (
    <div className="flex flex-col gap-3">
      {fields.map(([field, labelKey, options]) => {
        const id = `tactics-dial-${field}`;
        return (
          <div key={field} className="flex flex-col gap-1">
            <label htmlFor={id} className="text-caption font-semibold text-fg-muted">
              {t(`tactics.phaseSettings.${labelKey}`)}
            </label>
            <Select
              id={id}
              fullWidth
              value={(tacticsPhase?.[field] ?? options[0]) as string}
              onChange={(event) => {
                onTacticsPhaseChange({ [field]: event.target.value });
              }}
            >
              {options.map((option) => (
                <option key={option} value={option}>
                  {t(`tactics.phaseSettings.${labelKey}_${option}`, option)}
                </option>
              ))}
            </Select>
          </div>
        );
      })}
    </div>
  );
}

/**
 * The team instructions, on the left of the Tactics workbench: an accordion of the three phases
 * and the set pieces. It folds to a rail that keeps each section a click away.
 */
export default function TacticsInstructionsPanel({
  open,
  openSections,
  onOpenChange,
  onToggleSection,
  onOpenAt,
  tacticsPhase,
  onTacticsPhaseChange,
  setPieces,
}: TacticsInstructionsPanelProps): JSX.Element {
  const { t } = useTranslation();

  if (!open) {
    return (
      <nav
        aria-label={t("tactics.instructionsPanel.title")}
        className="flex h-full flex-col overflow-y-auto rounded-lg border border-line bg-panel"
      >
        <button
          type="button"
          aria-expanded={false}
          aria-label={t("tactics.instructionsPanel.show")}
          title={t("tactics.instructionsPanel.show")}
          onClick={() => onOpenChange(true)}
          className="flex h-panel-header shrink-0 items-center justify-center text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
        >
          <ChevronsRight className="h-4 w-4" aria-hidden="true" />
        </button>
        {SECTION_ORDER.map((section) => (
          <button
            key={section}
            type="button"
            aria-label={t("tactics.instructionsPanel.openAt", {
              section: t(SECTION_TITLE_KEYS[section]),
            })}
            onClick={() => onOpenAt(section)}
            className="flex justify-center border-t border-line py-3 text-fg hover:bg-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
          >
            <span className="rotate-180 font-heading text-caption font-bold uppercase tracking-label [writing-mode:vertical-rl]">
              {t(SECTION_TITLE_KEYS[section])}
            </span>
          </button>
        ))}
      </nav>
    );
  }

  return (
    <section
      id={PANEL_ID}
      aria-labelledby={INSTRUCTIONS_TITLE_ID}
      className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg border border-line bg-panel"
    >
      <div className="flex h-panel-header shrink-0 items-center gap-2 border-b border-line pl-4 pr-2">
        <h2
          id={INSTRUCTIONS_TITLE_ID}
          className="flex-1 font-heading text-heading font-bold uppercase tracking-label text-fg"
        >
          {t("tactics.instructionsPanel.title")}
        </h2>
        <button
          type="button"
          aria-expanded={true}
          aria-controls={PANEL_ID}
          aria-label={t("tactics.instructionsPanel.hide")}
          title={t("tactics.instructionsPanel.hide")}
          onClick={() => onOpenChange(false)}
          className="rounded-md p-2 text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
        >
          <ChevronsLeft className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {SECTION_ORDER.map((section) => {
          const expanded = openSections.has(section);
          const bodyId = `${PANEL_ID}-${section}`;
          return (
            <div key={section} className="border-b border-line last:border-b-0">
              <button
                type="button"
                aria-expanded={expanded}
                aria-controls={bodyId}
                onClick={() => onToggleSection(section)}
                className="flex min-h-row w-full items-center justify-between gap-2 bg-sunken px-4 text-left font-heading text-body font-bold uppercase tracking-label text-fg hover:bg-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus"
              >
                {t(SECTION_TITLE_KEYS[section])}
                <ChevronDown
                  aria-hidden="true"
                  className={`h-4 w-4 text-fg-muted transition-transform motion-reduce:transition-none ${expanded ? "" : "-rotate-90"}`}
                />
              </button>
              {expanded ? (
                <div id={bodyId} className="px-4 py-3">
                  {section === "setPieces" ? (
                    setPieces
                  ) : (
                    <PhaseFields
                      section={section}
                      tacticsPhase={tacticsPhase}
                      onTacticsPhaseChange={onTacticsPhaseChange}
                    />
                  )}
                </div>
              ) : null}
            </div>
          );
        })}
      </div>
    </section>
  );
}
