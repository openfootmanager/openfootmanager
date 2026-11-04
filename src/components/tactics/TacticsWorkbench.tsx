import type { JSX, ReactNode } from "react";
import { useState } from "react";
import type { TacticsPhaseSettings } from "../../store/types";
import TacticsInstructionsPanel, { type InstructionsSection } from "./TacticsInstructionsPanel";

/**
 * Whether the instructions panel shows: `auto` follows the room the workbench has (open beside the
 * lineup when it is wide, a rail when it is not); the manager's own choice then holds.
 */
type PanelMode = "auto" | "open" | "closed";

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
 * rests as a rail and opens over the lineup.
 */
export default function TacticsWorkbench({
  tacticsPhase,
  onTacticsPhaseChange,
  setPieces,
  children,
}: TacticsWorkbenchProps): JSX.Element {
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

  // The first column is the rail's width, or the panel's where the panel sits beside the lineup.
  const columns =
    mode === "closed"
      ? "grid-cols-[3rem_minmax(0,1fr)]"
      : "grid-cols-[3rem_minmax(0,1fr)] @min-[72rem]/workbench:grid-cols-[17rem_minmax(0,1fr)]";

  return (
    <div className="@container/workbench flex min-h-0 flex-1">
      <div
        data-instructions={mode}
        className={`relative grid min-h-0 w-full flex-1 gap-4 ${columns}`}
      >
        {mode === "auto" ? (
          // Both forms are drawn and the container query shows one; the hidden one is display:none,
          // so neither the keyboard nor a screen reader reaches it.
          <>
            <div className="min-h-0 @min-[72rem]/workbench:hidden">{panel(false)}</div>
            <div className="hidden min-h-0 @min-[72rem]/workbench:block">{panel(true)}</div>
          </>
        ) : null}
        {mode === "closed" ? <div className="min-h-0">{panel(false)}</div> : null}
        {mode === "open" ? (
          // Too narrow for both, the open panel lies over the lineup instead of squeezing it.
          <div className="absolute inset-y-0 left-0 z-20 w-[17rem] shadow-card @min-[72rem]/workbench:static @min-[72rem]/workbench:w-auto @min-[72rem]/workbench:shadow-none">
            {panel(true)}
          </div>
        ) : null}
        <div className="col-start-2 min-h-0 overflow-y-auto">{children}</div>
      </div>
    </div>
  );
}
