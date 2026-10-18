export interface ChartTheme {
  primary: string;
  secondary: string;
  danger: string;
  success: string;
  gridColor: string;
  axisColor: string;
  textColor: string;
  tooltipBg: string;
  tooltipBorder: string;
  tooltipText: string;
  tickFontSize: string;
}

// Every value is a role token. SVG attributes resolve var() in WebKitGTK and Chromium, so a chart
// follows light, dark and high contrast through CSS alone, with no colour kept in TypeScript.
const CHART_THEME: ChartTheme = {
  primary: "var(--color-series-1)",
  secondary: "var(--color-series-2)",
  danger: "var(--color-danger-solid)",
  success: "var(--color-positive-solid)",
  gridColor: "var(--color-line)",
  axisColor: "var(--color-fg-subtle)",
  textColor: "var(--color-fg-muted)",
  tooltipBg: "var(--color-panel)",
  tooltipBorder: "var(--color-line)",
  tooltipText: "var(--color-fg)",
  tickFontSize: "var(--text-caption)",
};

export function useChartTheme(): ChartTheme {
  return CHART_THEME;
}
