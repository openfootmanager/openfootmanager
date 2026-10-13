import type { PackageProjectData } from "../menu/PackageEditor/types";

/** Whether a validation reply has anything to show: errors, or warnings that leave the package usable. */
export function findingsToShow(data: Pick<PackageProjectData, "issues" | "warnings">): boolean {
  return data.issues.length > 0 || (data.warnings?.length ?? 0) > 0;
}
