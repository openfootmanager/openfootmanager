import type { JSX, ReactNode } from "react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useWidthAtLeast } from "../../hooks/useWidthAtLeast";
import { Button, Dialog } from "../ui";

/** The width at which the pitch sits beside the table: room for the table's compact columns and the pitch (20rem). */
const PITCH_BESIDE_REM = 56;

const PITCH_TITLE_ID = "tactics-pitch-title";

interface TacticsLineupFrameProps {
  /** The player table: it scrolls inside itself. */
  table: ReactNode;
  /** The pitch. */
  pitch: ReactNode;
}

/**
 * The lineup region of the Tactics workbench: the player table, and the pitch. It fills the height it
 * is given and never scrolls as a whole; the table and the pitch scroll inside themselves. Where
 * there is room for both (56rem or wider) the pitch sits beside the table; where there is not, the
 * table has the width and the pitch is behind a Pitch button that opens it as a panel over the table,
 * as the instructions rail does (maintainer, 11 Nov 2026).
 */
export default function TacticsLineupFrame({ table, pitch }: TacticsLineupFrameProps): JSX.Element {
  const { t } = useTranslation();
  const frameRef = useRef<HTMLDivElement>(null);
  const beside = useWidthAtLeast(frameRef, PITCH_BESIDE_REM);
  const [panelRequested, setPanelRequested] = useState(false);
  const panelOpen = !beside && panelRequested;

  return (
    <div ref={frameRef} className="flex h-full min-h-0 flex-col gap-2">
      {beside ? null : (
        <div className="flex shrink-0 justify-end">
          <Button
            type="button"
            variant="outline"
            size="sm"
            aria-haspopup="dialog"
            aria-expanded={panelOpen}
            onClick={() => setPanelRequested(true)}
          >
            {t("tactics.pitch")}
          </Button>
        </div>
      )}
      <div className="flex min-h-0 flex-1 gap-4">
        <div className="min-h-0 min-w-0 flex-1 overflow-hidden rounded-lg border border-line bg-panel">
          {table}
        </div>
        {beside ? <div className="min-h-0 w-80 shrink-0 overflow-y-auto">{pitch}</div> : null}
      </div>
      {panelOpen ? (
        <Dialog
          labelledBy={PITCH_TITLE_ID}
          onClose={() => setPanelRequested(false)}
          dismissOnBackdrop
          overlayClassName="bg-overlay"
          className="ml-auto h-full w-[22rem] max-w-[90vw] self-stretch overflow-y-auto"
        >
          <h2 id={PITCH_TITLE_ID} className="sr-only">
            {t("tactics.pitch")}
          </h2>
          {pitch}
        </Dialog>
      ) : null}
    </div>
  );
}
