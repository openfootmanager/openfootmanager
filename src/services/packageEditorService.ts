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

/**
 * What a save sends. A collection left out is untouched on disk; one sent as an empty list is written
 * empty. The two are different requests, so a caller never fills a missing collection with `[]`.
 */
export interface SavePackageRequest<Collections> {
  dir: string;
  collections: Partial<Collections>;
}

/** The collections a save wrote: only these may be marked clean. */
export interface SavePackageReceipt {
  saved: string[];
  filesWritten: number;
}

/** Save the collections of the open project that were sent; see [`SavePackageRequest`]. */
export function savePackageProject<Collections>(
  request: SavePackageRequest<Collections>,
): Promise<SavePackageReceipt> {
  return invoke<SavePackageReceipt>("save_package_project", {
    request: { dir: request.dir, ...request.collections },
  });
}
