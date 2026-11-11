import { FastForward, Pause, Play, SkipForward, Snail, StepForward } from "lucide-react";
import { useEffect, useId } from "react";
import { useTranslation } from "react-i18next";

import { Button, SegmentedControl } from "../ui";

/** How fast a live match plays when it isn't paused; `instant` runs to the end of the half. */
export type MatchPace = "slow" | "normal" | "fast" | "instant";

const PACE_LABEL_KEYS: Record<MatchPace, string> = {
  slow: "match.slow",
  normal: "match.normal",
  fast: "match.fast",
  instant: "match.max",
};

/**
 * Whether the focused element handles Space itself: a field types a space and a button or radio
 * presses itself, so the match-wide shortcut must not also fire.
 */
function ownsSpace(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  return (
    ["INPUT", "TEXTAREA", "SELECT", "BUTTON"].includes(target.tagName) ||
    ["button", "radio", "checkbox", "textbox"].includes(target.getAttribute("role") ?? "")
  );
}

interface MatchSpeedControlProps {
  paused: boolean;
  /** The pace the match plays at, or resumes at when paused. */
  pace: MatchPace;
  onTogglePause: () => void;
  onSelectPace: (pace: MatchPace) => void;
  onStep: () => void;
  /** Whether Space toggles the match; off while a dialog is open or the screen hands over. */
  enabled?: boolean;
  /** The step button's words, where a step is not a minute (a shootout steps one kick). */
  stepLabel?: string;
}

/**
 * The live match's speed, media-player style: one big Pause/Resume (Space from anywhere on the
 * screen), the four speeds as a radio group with an icon and a word each, and Step 1 minute while
 * paused. The chosen speed is filled while the match plays and only raised while it is paused, so
 * the two never look alike.
 */
export function MatchSpeedControl({
  paused,
  pace,
  onTogglePause,
  onSelectPace,
  onStep,
  enabled = true,
  stepLabel,
}: MatchSpeedControlProps) {
  const { t } = useTranslation();
  const headingId = useId();
  const paceName = t(PACE_LABEL_KEYS[pace]);

  useEffect(() => {
    if (!enabled) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.code !== "Space" && event.key !== " ") return;
      // A held key repeats, which would flicker the match; a modified Space is someone else's.
      if (event.repeat || event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) return;
      if (ownsSpace(event.target)) return;
      event.preventDefault();
      onTogglePause();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onTogglePause, enabled]);

  return (
    <div className="flex flex-col gap-3">
      <h3
        id={headingId}
        className="font-heading text-caption font-bold uppercase tracking-label text-fg-muted"
      >
        {t("match.simSpeed")}
      </h3>
      <button
        type="button"
        onClick={onTogglePause}
        aria-keyshortcuts="Space"
        // Three columns keep the word centred while the key hint sits at the end, and a long word
        // ("Fortsetzen" beside "Leertaste") pushes the hint aside instead of running under it.
        className="grid h-11 w-full grid-cols-[1fr_auto_1fr] items-center gap-2 rounded-lg bg-primary-700 px-3 font-heading text-lead font-bold uppercase tracking-label text-on-brand shadow-sm transition-colors hover:bg-primary-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
      >
        <span className="col-start-2 flex items-center gap-2">
          {paused ? (
            <Play className="h-5 w-5" fill="currentColor" aria-hidden="true" />
          ) : (
            <Pause className="h-5 w-5" fill="currentColor" aria-hidden="true" />
          )}
          {paused ? t("match.resume") : t("match.pause")}
        </span>
        <span
          aria-hidden="true"
          className="justify-self-end rounded border border-on-brand/50 px-1.5 py-0.5 font-sans text-caption font-semibold normal-case tracking-normal"
        >
          {t("match.spaceKey")}
        </span>
      </button>
      <p role="status" className="text-caption text-fg-muted">
        {paused
          ? t("match.pausedResumesAt", { speed: paceName })
          : t("match.playingAt", { speed: paceName })}
      </p>
      <SegmentedControl
        labelledBy={headingId}
        options={[
          { value: "slow", label: t("match.slow"), icon: <Snail /> },
          { value: "normal", label: t("match.normal"), icon: <Play /> },
          { value: "fast", label: t("match.fast"), icon: <FastForward /> },
          { value: "instant", label: t("match.max"), icon: <SkipForward /> },
        ]}
        value={pace}
        onChange={onSelectPace}
        emphasis={paused ? "outlined" : "strong"}
        // Wrapping from Slow (or past Fast) would land on Max, which runs the match to half time.
        wrap={false}
        layout="stacked"
        fill
      />
      <Button
        variant="outline"
        size="sm"
        icon={<StepForward />}
        onClick={onStep}
        disabled={!paused}
        className="h-row w-full"
      >
        {stepLabel ?? t("match.step1Min")}
      </Button>
    </div>
  );
}
