import { AlertCircle, ChevronRight } from "lucide-react";
import type { JSX } from "react";

import type { DashboardAlert } from "./dashboardAlertState";

interface DashboardAlertsProps {
  alerts: DashboardAlert[];
  onNavigate: (tab: string) => void;
}

function getAlertButtonClassName(severity: DashboardAlert["severity"]): string {
  const baseClassName =
    "flex items-center gap-2 rounded-lg border px-4 py-2 text-xs font-heading font-bold uppercase tracking-wider transition-all";

  if (severity === "warn") {
    return `${baseClassName} border-amber-500/20 bg-amber-500/10 text-amber-600 hover:bg-amber-500/20 dark:text-amber-400`;
  }

  return `${baseClassName} border-blue-500/20 bg-blue-500/10 text-blue-600 hover:bg-blue-500/20 dark:text-blue-400`;
}

export default function DashboardAlerts({
  alerts,
  onNavigate,
}: DashboardAlertsProps): JSX.Element | null {
  if (alerts.length === 0) {
    return null;
  }

  return (
    <div className="mb-4 flex flex-col gap-1.5">
      {alerts.map((alert) => (
        <button
          key={alert.id}
          onClick={() => onNavigate(alert.tab)}
          className={getAlertButtonClassName(alert.severity)}
        >
          <AlertCircle className="h-3.5 w-3.5 shrink-0" />
          <span className="flex-1 text-left">{alert.text}</span>
          <ChevronRight className="h-3 w-3" />
        </button>
      ))}
    </div>
  );
}
