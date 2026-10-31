import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { MatchSpeedControl } from "./MatchSpeedControl";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, string>) =>
      options?.speed ? `${key}(${options.speed})` : key,
  }),
}));

function renderControl(overrides: Partial<React.ComponentProps<typeof MatchSpeedControl>> = {}) {
  const props = {
    paused: false,
    pace: "normal" as const,
    onTogglePause: vi.fn(),
    onSelectPace: vi.fn(),
    onStep: vi.fn(),
    ...overrides,
  };
  render(<MatchSpeedControl {...props} />);
  return props;
}

describe("MatchSpeedControl", () => {
  /**
   * Given a match playing at normal speed
   * When the control renders
   * Then the big toggle reads Pause with Space as its shortcut, the line says the pace, the
   * speeds are a radio group named by the visible heading with Normal checked, and Step is off.
   */
  it("shows a playing match: Pause, the pace, and no step", () => {
    renderControl();
    const toggle = screen.getByRole("button", { name: "match.pause" });
    expect(toggle).toHaveAttribute("aria-keyshortcuts", "Space");
    expect(screen.getByRole("status")).toHaveTextContent("match.playingAt(match.normal)");
    expect(screen.getByRole("radiogroup", { name: "match.simSpeed" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "match.normal" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("button", { name: "match.step1Min" })).toBeDisabled();
  });

  /**
   * Given a paused match that will resume at fast speed
   * When the control renders
   * Then the toggle reads Resume, the line says it resumes at Fast, and Step is on.
   */
  it("shows a paused match: Resume, where it resumes, and the step", () => {
    renderControl({ paused: true, pace: "fast" });
    expect(screen.getByRole("button", { name: "match.resume" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("match.pausedResumesAt(match.fast)");
    expect(screen.getByRole("button", { name: "match.step1Min" })).toBeEnabled();
  });

  /**
   * Given a paused match
   * When the control renders
   * Then the speed it resumes at stays outlined, not filled (that means playing) and not a faint
   * grey ring; and Step is a full row tall, like every target in the panel.
   */
  it("outlines the pace a paused match resumes at, and Step is a row tall", () => {
    renderControl({ paused: true, pace: "fast" });
    const fast = screen.getByRole("radio", { name: "match.fast" });
    expect(fast).toHaveClass("ring-selected");
    expect(fast).not.toHaveClass("bg-primary-700");
    expect(screen.getByRole("button", { name: "match.step1Min" })).toHaveClass("h-row");
  });

  /**
   * Given Fast chosen, or Slow
   * When the arrow keys go past the end of the speeds
   * Then the choice stays put: wrapping would land on Max, which runs the match to half time and
   * can't be undone.
   */
  it("doesn't wrap the arrows round to Max", () => {
    const props = renderControl({ pace: "instant" });
    fireEvent.keyDown(screen.getByRole("radio", { name: "match.max" }), { key: "ArrowRight" });
    expect(props.onSelectPace).not.toHaveBeenCalled();
  });

  it("doesn't wrap from Slow back to Max", () => {
    const props = renderControl({ pace: "slow" });
    fireEvent.keyDown(screen.getByRole("radio", { name: "match.slow" }), { key: "ArrowLeft" });
    expect(props.onSelectPace).not.toHaveBeenCalled();
  });

  /**
   * Given the control switched off (a dialog is open over the match, or it is handing over to the
   * break)
   * When Space is pressed
   * Then nothing toggles.
   */
  it("leaves Space alone when switched off", () => {
    const props = renderControl({ enabled: false });
    fireEvent.keyDown(document.body, { key: " ", code: "Space" });
    expect(props.onTogglePause).not.toHaveBeenCalled();
  });

  /**
   * Given the control has left the screen (the match moved on to half time)
   * When Space is pressed
   * Then nothing toggles: the shortcut goes with the control.
   */
  it("stops listening for Space once it is gone", () => {
    const onTogglePause = vi.fn();
    const { unmount } = render(
      <MatchSpeedControl
        paused={false}
        pace="normal"
        onTogglePause={onTogglePause}
        onSelectPace={vi.fn()}
        onStep={vi.fn()}
      />,
    );
    unmount();
    fireEvent.keyDown(document.body, { key: " ", code: "Space" });
    expect(onTogglePause).not.toHaveBeenCalled();
  });

  /** Given the control, when the toggle, a speed and the step are pressed, then each is reported. */
  it("reports the toggle, a chosen speed and a step", () => {
    const props = renderControl({ paused: true });
    fireEvent.click(screen.getByRole("button", { name: "match.resume" }));
    fireEvent.click(screen.getByRole("radio", { name: "match.max" }));
    fireEvent.click(screen.getByRole("button", { name: "match.step1Min" }));
    expect(props.onTogglePause).toHaveBeenCalledTimes(1);
    expect(props.onSelectPace).toHaveBeenCalledWith("instant");
    expect(props.onStep).toHaveBeenCalledTimes(1);
  });

  /**
   * Given nothing focused on the match screen
   * When Space is pressed
   * Then the match pauses or resumes, as the toggle says.
   */
  it("toggles on Space from the page", () => {
    const props = renderControl();
    fireEvent.keyDown(document.body, { key: " ", code: "Space" });
    expect(props.onTogglePause).toHaveBeenCalledTimes(1);
  });

  /**
   * Given nothing focused on the match screen
   * When Space is held down (the key repeats) or pressed with a modifier
   * Then the match isn't toggled again: a held key would flicker it, and a modified Space belongs
   * to the system or another shortcut.
   */
  it("ignores a held or modified Space", () => {
    const props = renderControl();
    fireEvent.keyDown(document.body, { key: " ", code: "Space", repeat: true });
    fireEvent.keyDown(document.body, { key: " ", code: "Space", ctrlKey: true });
    fireEvent.keyDown(document.body, { key: " ", code: "Space", shiftKey: true });
    fireEvent.keyDown(document.body, { key: " ", code: "Space", altKey: true });
    fireEvent.keyDown(document.body, { key: " ", code: "Space", metaKey: true });
    expect(props.onTogglePause).not.toHaveBeenCalled();
  });

  /**
   * Given the focus in a text field or on a button
   * When Space is pressed
   * Then the shortcut leaves it alone: the field types a space and the button presses itself.
   */
  it("leaves Space to a focused field or button", () => {
    const props = renderControl();
    const field = document.createElement("input");
    document.body.append(field);
    fireEvent.keyDown(field, { key: " ", code: "Space" });
    fireEvent.keyDown(screen.getByRole("radio", { name: "match.fast" }), {
      key: " ",
      code: "Space",
    });
    expect(props.onTogglePause).not.toHaveBeenCalled();
    field.remove();
  });
});
