import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { buildSchema, parse, validate } from "graphql";
import { afterEach, describe, expect, it } from "vitest";

const root = resolve(__dirname, "../../..");
const script = resolve(root, "scripts/generate-entity-documents.mjs");
const temporary: string[] = [];
const BASE = `
type Query {
  things(where: ThingWhereInput, page: PageInput): ThingConnection!
  thing(id: ID!): Thing
}
type Thing { id: ID!, label: String! }
input ThingWhereInput { label: String }
input PageInput { limit: Int, offset: Int }
type ThingConnection { edges: [ThingEdge!]!, pageInfo: PageInfo! }
type ThingEdge { node: Thing!, cursor: String! }
type PageInfo { hasNextPage: Boolean!, hasPreviousPage: Boolean!, totalCount: Int, startCursor: String, endCursor: String }
`;
function fixture(schema = BASE) {
  const dir = mkdtempSync(resolve(tmpdir(), "librarian-entity-generator-")); temporary.push(dir);
  const source = resolve(dir, "schema.graphql"), output = resolve(dir, "entities");
  writeFileSync(source, schema);
  const run = (...args: string[]) => spawnSync(process.execPath, [script, "--schema", source, "--output", output, ...args], { cwd: root, encoding: "utf8" });
  return { source, output, run };
}
afterEach(() => { temporary.splice(0).forEach((path) => rmSync(path, { recursive: true, force: true })); });

describe("entity document generation", () => {
  it("supports a read-only schema and preserves its actual ID argument type", () => {
    const test = fixture(); expect(test.run().status).toBe(0);
    const document = readFileSync(resolve(test.output, "thing.graphql"), "utf8");
    expect(document).toContain("$id: ID!");
    expect(document).not.toContain("mutation "); expect(document).not.toContain("subscription ");
    expect(validate(buildSchema(BASE), parse(document))).toEqual([]);
    expect(JSON.parse(readFileSync(resolve(test.output, "coverage.json"), "utf8"))).toEqual([{ entity: "Thing", list: "things", get: "thing", changed: null }]);
  });

  it("generates keysets separately without overwriting the canonical list", () => {
    const schema = BASE.replace("  thing(id:", "  thingsKeyset(where: ThingWhereInput, page: KeysetPageInput!): ThingConnection!\n  thing(id:")
      + "\ninput KeysetPageInput { limit: Int, after: String }\n";
    const test = fixture(schema); expect(test.run().status).toBe(0);
    const document = readFileSync(resolve(test.output, "thing.graphql"), "utf8");
    expect(document).toContain("query EntityThingList"); expect(document).toContain("query EntityThingKeyset");
    expect(validate(buildSchema(schema), parse(document))).toEqual([]);
  });

  it("checks drift without rewriting the reviewable output", () => {
    const test = fixture(); expect(test.run().status).toBe(0);
    expect(test.run("--check").status).toBe(0);
    const file = resolve(test.output, "thing.graphql"); writeFileSync(file, "# deliberate drift\n");
    const checked = test.run("--check"); expect(checked.status).not.toBe(0);
    expect(checked.stderr).toContain("Generated documents differ");
    expect(readFileSync(file, "utf8")).toBe("# deliberate drift\n");
  });

  it("rejects ambiguous canonical roots before deleting existing documents", () => {
    const test = fixture(); expect(test.run().status).toBe(0);
    const file = resolve(test.output, "thing.graphql"), before = readFileSync(file, "utf8");
    writeFileSync(test.source, BASE.replace("  thing(id:", "  duplicateThings(where: ThingWhereInput, page: PageInput): ThingConnection!\n  thing(id:"));
    const checked = test.run(); expect(checked.status).not.toBe(0); expect(checked.stderr).toContain("Ambiguous");
    expect(readFileSync(file, "utf8")).toBe(before);
  });
});
