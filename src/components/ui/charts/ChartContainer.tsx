import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

interface ChartContainerProps {
  children?: ReactNode;
  height?: number;
  isEmpty?: boolean;
  className?: string;
}

export function ChartContainer({
  children,
  height = 220,
  isEmpty = false,
  className = "",
}: ChartContainerProps) {
  const { t } = useTranslation();

  if (isEmpty) {
    return (
      <div
        className={`flex items-center justify-center rounded-lg bg-sunken ${className}`}
        style={{ height }}
      >
        <p className="text-caption text-fg-subtle">{t("common.noChartData")}</p>
      </div>
    );
  }

  return (
    <div className={`w-full ${className}`} style={{ height }}>
      {children}
    </div>
  );
}
