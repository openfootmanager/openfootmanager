import { useContext } from "react";
import { ThemeContext } from "../../../context/ThemeContext";

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
}

const DARK_THEME: ChartTheme = {
  primary: "#10b981",
  secondary: "#6366f1",
  danger: "#ef4444",
  success: "#06d6a0",
  gridColor: "#243054",
  axisColor: "#6b7280",
  textColor: "#d1d5db",
  tooltipBg: "#1a2340",
  tooltipBorder: "#243054",
  tooltipText: "#f3f4f6",
};

const LIGHT_THEME: ChartTheme = {
  primary: "#059669",
  secondary: "#4f46e5",
  danger: "#dc2626",
  success: "#05a87d",
  gridColor: "#e5e7eb",
  axisColor: "#9ca3af",
  textColor: "#374151",
  tooltipBg: "#ffffff",
  tooltipBorder: "#e5e7eb",
  tooltipText: "#111827",
};

export function useChartTheme(): ChartTheme {
  const ctx = useContext(ThemeContext);
  const isDark = ctx?.isDark ?? true;
  return isDark ? DARK_THEME : LIGHT_THEME;
}
