import type { ReactNode } from "react";

import type { LogFileSummary } from "../../services/reportService";
import { formatBytes } from "./ReportBugModal.helpers";

interface PreviewRowProps {
  icon: ReactNode;
  name: string;
  detail: string;
  size?: string;
  files?: LogFileSummary[];
}

export function PreviewRow({ icon, name, detail, size, files }: PreviewRowProps) {
  return (
    <div className="flex items-start gap-2.5 px-3 py-2.5 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600">
      <span className="text-primary-600 dark:text-primary-400 shrink-0 mt-0.5">{icon}</span>
      <div className="flex-1 min-w-0">
        <p className="text-[13px] font-medium text-gray-800 dark:text-gray-200">{name}</p>
        <p className="text-[11px] text-gray-600 dark:text-gray-400 mt-0.5">{detail}</p>
        {files && files.length > 0 && (
          <ul className="mt-1.5 flex flex-col gap-0.5">
            {files.map((file) => (
              <li
                key={file.name}
                className="flex items-baseline justify-between gap-3 text-[11px] font-mono text-gray-500 dark:text-gray-400"
              >
                <span className="truncate">{file.name}</span>
                <span className="shrink-0">{formatBytes(file.bytes)}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
      {size && (
        <span className="text-[11px] font-mono text-gray-600 dark:text-gray-400 shrink-0">
          {size}
        </span>
      )}
    </div>
  );
}
