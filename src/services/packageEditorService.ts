import { invoke } from "@tauri-apps/api/core";

import type { PackageCollections } from "../store/packageEditorTypes";
import { recordValidation } from "./recentProjects";

export interface WorldLibraryRow {
  path: string;
  name: string;
  version: string | null;
  clubs: number;
  players: number;
  countries: number;
  error: string | null;
}

export interface WorldsLibrary {
  projects: WorldLibraryRow[];
  installed: WorldLibraryRow[];
}

export function getWorldsLibrary(paths: string[]): Promise<WorldsLibrary> {
  return invoke<WorldsLibrary>("get_worlds_library", { paths });
}

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
export async function validatePackageProject(dir: string): Promise<PackageDiagnostics> {
  const result = await invoke<PackageDiagnostics>("validate_package_project", { dir });
  recordValidation(dir, result.issues.length, result.warnings.length);
  return result;
}

/**
 * What a save sends. A collection left out is untouched on disk; one sent as an empty list is written
 * empty, so a caller never fills a missing collection with `[]`. Typed against the package's real
 * collection types, with no type parameter to name some other shape.
 */
export interface SavePackageRequest {
  dir: string;
  collections: Partial<PackageCollections>;
}

/** The collections a save wrote: only these may be marked clean. */
export interface SavePackageReceipt {
  saved: string[];
  filesWritten: number;
}

/** Save the collections of the open project that were sent; see [`SavePackageRequest`]. */
export function savePackageProject(request: SavePackageRequest): Promise<SavePackageReceipt> {
  return invoke<SavePackageReceipt>("save_package_project", {
    request: { dir: request.dir, ...request.collections },
  });
}
