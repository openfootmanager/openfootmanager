import type { JSX, ReactNode } from "react";
import { useRef, useState } from "react";
import { useWidthAtLeast } from "../../hooks/useWidthAtLeast";
import type { TacticsPhaseSettings } from "../../store/types";
import { Dialog } from "../ui";
import TacticsInstructionsPanel, {
  INSTRUCTIONS_TITLE_ID,
  type InstructionsSection,
} from "./TacticsInstructionsPanel";

/**
 * Whether the instructions panel shows: `auto` follows the room the workbench has (open beside the
 * lineup when it is wide, a rail when it is not); the manager's own choice then holds.
 */
type PanelMode = "auto" | "open" | "closed";

/** The width at which the panel sits beside the lineup (a 1440 window beside the sidebar). */
const PANEL_BESIDE_REM = 72;

interface TacticsWorkbenchProps {
  tacticsPhase?: TacticsPhaseSettings;
  onTacticsPhaseChange: (patch: Partial<TacticsPhaseSettings>) => void;
  /** The set-piece takers' section body. */
  setPieces: ReactNode;
  /** The lineup: the region beside the panel, which scrolls inside itself. */
  children: ReactNode;
}

/**
 * The Tactics workbench: the team instructions on the left, the lineup beside them. It fills the
 * height it is given and never makes the page scroll; each region scrolls inside itself. Where the
 * workbench is too narrow for both (under 72rem, so in a 1280 window beside the sidebar), the panel
 * rests as a rail and opens as a drawer over the page (maintainer, 4 Nov 2026).
 */
export default function TacticsWorkbench({
  tacticsPhase,
  onTacticsPhaseChange,
  setPieces,
  children,
}: TacticsWorkbenchProps): JSX.Element {
  const workbenchRef = useRef<HTMLDivElement>(null);
  const wide = useWidthAtLeast(workbenchRef, PANEL_BESIDE_REM);
  const [mode, setMode] = useState<PanelMode>("auto");
  // Every section starts open, so nothing the screen offers starts hidden; the manager folds what
  // he doesn't need.
  const [openSections, setOpenSections] = useState<ReadonlySet<InstructionsSection>>(
    () => new Set<InstructionsSection>(["withBall", "transitions", "withoutBall", "setPieces"]),
  );

  function toggleSection(section: InstructionsSection): void {
    setOpenSections((current) => {
      const next = new Set(current);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      return next;
    });
  }

  function openAt(section: InstructionsSection): void {
    setOpenSections((current) => new Set(current).add(section));
    setMode("open");
  }

  const panel = (open: boolean) => (
    <TacticsInstructionsPanel
      open={open}
      openSections={openSections}
      onOpenChange={(next) => setMode(next ? "open" : "closed")}
      onToggleSection={toggleSection}
      onOpenAt={openAt}
      tacticsPhase={tacticsPhase}
      onTacticsPhaseChange={onTacticsPhaseChange}
      setPieces={setPieces}
    />
  );

  // Beside the lineup only where there is room for both; elsewhere the rail, and a drawer when open.
  const beside = wide && mode !== "closed";
  const drawerOpen = !wide && mode === "open";

  return (
    <div
      ref={workbenchRef}
      data-instructions={beside ? "beside" : drawerOpen ? "drawer" : "rail"}
      className={`grid min-h-0 w-full flex-1 gap-4 ${beside ? "grid-cols-[17rem_minmax(0,1fr)]" : "grid-cols-[3rem_minmax(0,1fr)]"}`}
    >
      <div className="min-h-0">{panel(beside)}</div>
      <div className="min-h-0 overflow-y-auto">{children}</div>
      {drawerOpen ? (
        <Dialog
          labelledBy={INSTRUCTIONS_TITLE_ID}
          onClose={() => setMode("closed")}
          dismissOnBackdrop
          overlayClassName="bg-overlay"
          className="mr-auto h-full w-[18rem] max-w-[85vw] self-stretch"
        >
          {panel(true)}
        </Dialog>
      ) : null}
    </div>
  );
}
