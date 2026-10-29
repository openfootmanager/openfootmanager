import { AlertCircle, ChevronRight } from "lucide-react";
import type { JSX } from "react";

import type { DashboardAlert } from "./dashboardHelpers";

interface DashboardAlertsProps {
  alerts: DashboardAlert[];
  onNavigate: (tab: string) => void;
}

// The alert is a sentence, so it reads in sentence case; the tint says how urgent, the icon and
// words say what.
const ALERT_BASE =
  "flex items-center gap-2 rounded-lg border border-line px-4 py-2 text-body font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page";

function getAlertButtonClassName(severity: DashboardAlert["severity"]): string {
  if (severity === "warn") {
    return `${ALERT_BASE} bg-warning-tint text-warning hover:border-warning-solid`;
  }

  return `${ALERT_BASE} bg-info-tint text-info hover:border-info-solid`;
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
          type="button"
          key={alert.id}
          onClick={() => onNavigate(alert.tab)}
          className={getAlertButtonClassName(alert.severity)}
        >
          <AlertCircle className="h-4 w-4 shrink-0" />
          <span className="flex-1 text-left">{alert.text}</span>
          <ChevronRight className="h-4 w-4 shrink-0" />
        </button>
      ))}
    </div>
  );
}
