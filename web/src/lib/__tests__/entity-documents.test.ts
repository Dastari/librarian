import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { buildSchema, parse, validate } from "graphql";
import { describe, expect, it } from "vitest";

/** Every operation document (generated and hand-written) must validate against the schema snapshot. */
describe("GraphQL documents", () => {
  const root = resolve(__dirname, "../../..");
  const schema = buildSchema(readFileSync(resolve(root, "schema.graphql"), "utf8"));

  const documents = ["src/graphql/entities", "src/graphql/documents"].flatMap((dir) => readdirSync(resolve(root, dir)).filter((file) => file.endsWith(".graphql")).map((file) => resolve(root, dir, file)));

  it("covers every generated entity", () => {
    expect(documents.length).toBeGreaterThan(45);
  });

  it("validates against the schema", () => {
    const merged = parse(documents.map((file) => readFileSync(file, "utf8")).join("\n"));
    const errors = validate(schema, merged);
    expect(errors.map((error) => error.message)).toEqual([]);
  });
});
