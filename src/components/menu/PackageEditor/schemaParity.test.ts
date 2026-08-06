import { describe, it, expect } from "vitest";
// Vite's raw import: the file's text, resolved the same way any other import in
// this directory is. Interfaces are erased before the test runs, so reading the
// source is the only way to see them.
import typesSource from "./types.ts?raw";
import schemaFields from "./schemaFields.generated.json";

/**
 * The editor's half of the package-schema handshake.
 *
 * `schemaFields.generated.json` is written from the Rust definitions by
 * `ofm_core::generator::scaffold`'s `the_frontend_fixture_…` test — the same
 * fully-populated instances the `ofm-cli` templates are checked against. This
 * asserts the editor's TypeScript types know every field in it.
 *
 * **What this does and does not claim.** Saving sends the loaded objects
 * straight back (`persist({ players: items })` in `WorldEditor.tsx`), and a
 * JavaScript object keeps properties TypeScript never declared — so a field the
 * editor cannot name still survives a round trip. Nothing is silently deleted.
 *
 * What is lost is the editor's ability to *see* the field: no form can show it,
 * no code can read it without a cast, and nobody adding a feature knows it
 * exists. `age` on a player sat in that state — a real, loadable `PlayerDef`
 * field the editor had no name for.
 *
 * The types are checked rather than the `empty*()` constructors, because those
 * legitimately leave optional fields out: an absent `fallbackLeague` means "use
 * the built-in defaults", which is not the same as `null`, and forcing every
 * optional field into a new entity would change what gets written.
 */

/** Which TypeScript interface carries each schema. */
const INTERFACE_FOR: Record<keyof typeof schemaFields, string> = {
  team: "TeamDef",
  player: "PlayerDef",
  staff: "StaffDef",
  confederation: "ConfederationDef",
  country: "CountryDef",
  competition: "CompetitionDef",
  world: "WorldMetaDef",
  // The one schema that is a single keyed document rather than a list of
  // entities, and the only one whose fields are snake_case.
  names: "NamesDefinition",
};

/**
 * Property names declared directly on `interface <name>`.
 *
 * The shape being matched is our own and simple: one property per line, two-space
 * indented, inside the interface body.
 */
function declaredFields(interfaceName: string): string[] {
  const body = new RegExp(`export interface ${interfaceName}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(
    typesSource,
  );
  if (!body) throw new Error(`interface ${interfaceName} not found in types.ts`);
  return [...body[1].matchAll(/^ {2}([A-Za-z][A-Za-z0-9]*)\??\s*:/gm)].map((m) => m[1]);
}

const SCHEMAS = Object.keys(INTERFACE_FOR) as Array<keyof typeof schemaFields>;

describe("package schema parity with the Rust definitions", () => {
  it.each(SCHEMAS)("the editor's %s type declares every serialized field", (schema) => {
    const declared = declaredFields(INTERFACE_FOR[schema]);
    const missing = (schemaFields[schema] as string[]).filter((f) => !declared.includes(f));

    expect(
      missing,
      `${INTERFACE_FOR[schema]} does not declare ${JSON.stringify(missing)}, which the ` +
        `package format defines. The editor cannot show or edit what it cannot name. ` +
        `Add them to types.ts.`,
    ).toEqual([]);
  });

  it("finds an interface for every schema the format defines", () => {
    // Guards against a new entity kind landing in Rust with no editor type at
    // all — the case the per-schema tests above would skip rather than fail.
    expect(SCHEMAS.slice().sort()).toEqual(Object.keys(schemaFields).sort());
  });
});
