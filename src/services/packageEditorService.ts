import { invoke } from "@tauri-apps/api/core";

/** One finding about a package project. Mirrors the backend's `PackageIssue`. */
export interface PackageDiagnostic {
  code: string;
  file: string;
  params: Record<string, string>;
}

/** What validating a project reports: its issues and warnings, never its entities. */
export interface PackageDiagnostics {
  issues: PackageDiagnostic[];
  warnings: PackageDiagnostic[];
}

/**
 * Validate the project on disk. The backend reads the folder again, so save first; the answer is the
 * diagnostics alone, because the editor already holds every entity and re-sending them is what made
 * Validate slow on a large package.
 */
export function validatePackageProject(dir: string): Promise<PackageDiagnostics> {
  return invoke<PackageDiagnostics>("validate_package_project", { dir });
}
