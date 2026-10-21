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
}: MatchSpeedControlProps) {
  const { t } = useTranslation();
  const headingId = useId();
  const paceName = t(PACE_LABEL_KEYS[pace]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.code !== "Space" && event.key !== " ") return;
      if (ownsSpace(event.target)) return;
      event.preventDefault();
      onTogglePause();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onTogglePause]);

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
        className="relative flex h-11 w-full items-center justify-center gap-2 rounded-lg bg-primary-700 font-heading text-lead font-bold uppercase tracking-label text-on-brand shadow-sm transition-colors hover:bg-primary-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
      >
        {paused ? (
          <Play className="h-5 w-5" fill="currentColor" aria-hidden="true" />
        ) : (
          <Pause className="h-5 w-5" fill="currentColor" aria-hidden="true" />
        )}
        {paused ? t("match.resume") : t("match.pause")}
        <kbd
          aria-hidden="true"
          className="absolute right-3 rounded border border-on-brand/50 px-1.5 py-0.5 font-sans text-caption font-semibold normal-case tracking-normal"
        >
          {t("match.spaceKey")}
        </kbd>
      </button>
      <p aria-live="polite" className="text-caption text-fg-muted">
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
        emphasis={paused ? "quiet" : "strong"}
        layout="stacked"
        fill
      />
      <Button
        variant="outline"
        size="sm"
        icon={<StepForward />}
        onClick={onStep}
        disabled={!paused}
        className="w-full"
      >
        {t("match.step1Min")}
      </Button>
    </div>
  );
}
