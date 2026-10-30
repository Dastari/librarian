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
    const coverage = JSON.parse(readFileSync(resolve(root, "src/graphql/entities/coverage.json"), "utf8")) as Array<{ entity: string; list: string; get: string | null; changed: string | null }>;
    const registered = JSON.parse(readFileSync(resolve(root, "../backend/tests/fixtures/authorization-matrix.json"), "utf8")) as Array<{ entity: string }>;
    expect(coverage.map(({ entity }) => entity).sort()).toEqual(registered.map(({ entity }) => entity).sort());
    for (const entry of coverage) {
      expect(schema.getQueryType()?.getFields()[entry.list]).toBeDefined();
      expect(entry.get).toBeTruthy();
      expect(schema.getQueryType()?.getFields()[entry.get!]).toBeDefined();
      expect(entry.changed).toBeTruthy();
      expect(schema.getSubscriptionType()?.getFields()[entry.changed!]).toBeDefined();
    }
  });

  it("keeps credential material out of outputs and filter/order surfaces", () => {
    const privateFields = {
      User: ["passwordHash", "password"],
      RefreshToken: ["tokenHash", "session", "grantSource", "roles", "authorizationRoles", "ipAddress", "userAgent"],
      InviteToken: ["token"],
      UsenetServer: ["password", "passwordNonce"],
      Source: ["credentials", "credentialsNonce"],
    };
    for (const [entity, forbidden] of Object.entries(privateFields)) {
      for (const name of [entity, `${entity}WhereInput`, `${entity}OrderByInput`]) {
        const type = schema.getType(name);
        if (!type || !("getFields" in type)) continue;
        const fields = Object.keys(type.getFields());
        for (const field of forbidden) expect(fields, `${name}.${field}`).not.toContain(field);
      }
    }
  });

  it("validates against the schema", () => {
    const merged = parse(documents.map((file) => readFileSync(file, "utf8")).join("\n"));
    const errors = validate(schema, merged);
    expect(errors.map((error) => error.message)).toEqual([]);
  });
});
