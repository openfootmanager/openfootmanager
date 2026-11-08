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

/** The collections of a package project a save can carry: the keys of the backend's request. */
export type SaveCollectionKey =
  | "meta"
  | "confederations"
  | "countries"
  | "teams"
  | "players"
  | "staff"
  | "names"
  | "competitions";

/**
 * What a save sends, typed against `C`, the caller's own collection types, which must be named
 * (`savePackageProject<EditorCollections>(...)`): `NoInfer` stops a literal such as `{ teams: 42 }`
 * or `{ team: [] }` from defining its own type. A collection left out is untouched on disk; one
 * sent as an empty list is written empty, so a caller never fills a missing collection with `[]`.
 */
export interface SavePackageRequest<C extends Record<SaveCollectionKey, unknown>> {
  dir: string;
  collections: NoInfer<Partial<C>>;
}

/** The collections a save wrote: only these may be marked clean. */
export interface SavePackageReceipt {
  saved: string[];
  filesWritten: number;
}

/** Save the collections of the open project that were sent; see [`SavePackageRequest`]. */
export function savePackageProject<C extends Record<SaveCollectionKey, unknown>>(
  request: SavePackageRequest<C>,
): Promise<SavePackageReceipt> {
  return invoke<SavePackageReceipt>("save_package_project", {
    request: { dir: request.dir, ...request.collections },
  });
}
