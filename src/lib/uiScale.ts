export type UiScale = "small" | "normal" | "large" | "xlarge";

/** The root font size each ui_scale setting applies. Every rem in the app follows it. */
const UI_SCALE_ROOT_FONT_SIZE: Record<UiScale, string> = {
  small: "14px",
  normal: "16px",
  large: "18px",
  xlarge: "20px",
};

export function rootFontSizeFor(scale: string): string {
  return UI_SCALE_ROOT_FONT_SIZE[scale as UiScale] ?? UI_SCALE_ROOT_FONT_SIZE.normal;
}
