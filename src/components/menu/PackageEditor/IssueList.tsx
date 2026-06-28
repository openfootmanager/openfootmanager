import { useTranslation } from "react-i18next";
import { AlertCircle } from "lucide-react";
import type { PackageIssue } from "./types";

export function IssueList({ issues }: { issues: PackageIssue[] }) {
  const { t } = useTranslation();
  if (issues.length === 0) return null;
  return (
    <div className="rounded-xl border border-red-300 dark:border-red-500/40 bg-red-50 dark:bg-red-500/10 p-3 text-xs">
      <p className="font-heading font-bold uppercase tracking-wider text-red-600 dark:text-red-400 mb-1 flex items-center gap-1">
        <AlertCircle className="w-3.5 h-3.5" />
        {t("worldEditor.issues", { count: issues.length })}
      </p>
      <ul className="list-disc pl-4 space-y-0.5 text-red-600 dark:text-red-300">
        {issues.map((issue, i) => (
          <li key={i}>
            {issue.file ? `[${issue.file}] ` : ""}
            {t(issue.code, issue.params)}
          </li>
        ))}
      </ul>
    </div>
  );
}
