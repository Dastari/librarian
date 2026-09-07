import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

interface IntrospectionTypeRef {
  kind: string;
  name: string | null;
  ofType?: IntrospectionTypeRef | null;
}

interface IntrospectionField {
  name: string;
  args: Array<{ name: string; type: IntrospectionTypeRef }>;
  type: IntrospectionTypeRef;
}

interface IntrospectionType {
  name: string;
  fields?: IntrospectionField[] | null;
}

const schema = JSON.parse(
  readFileSync(
    resolve(__dirname, "../../src/lib/graphql/generated/schema.json"),
    "utf8",
  ),
) as { __schema: { types: IntrospectionType[] } };

function type(name: string): IntrospectionType {
  const found = schema.__schema.types.find((entry) => entry.name === name);
  if (!found) throw new Error(`Missing GraphQL type ${name}`);
  return found;
}

function namedType(ref: IntrospectionTypeRef): string | null {
  return ref.name ?? (ref.ofType ? namedType(ref.ofType) : null);
}

describe("generated GraphQL schema contract", () => {
  it("contains the CastDevice capabilities used by the global cast client", () => {
    const fields = new Set(
      (type("CastDevice").fields ?? []).map((field) => field.name),
    );

    expect(fields.has("enabled")).toBe(true);
    expect(fields.has("playbackSupported")).toBe(true);
    expect(fields.has("discoveryOrigin")).toBe(true);
  });

  it("keeps login on the cookie-session AuthPayload contract", () => {
    const login = (type("Mutation").fields ?? []).find(
      (field) => field.name === "login",
    );

    expect(login).toBeDefined();
    expect(login?.args.map((argument) => argument.name)).toEqual(["input"]);
    expect(namedType(login!.args[0].type)).toBe("LoginInput");
    expect(namedType(login!.type)).toBe("AuthPayload");

    const sessionFields = new Set(
      (type("AuthSessionInfo").fields ?? []).map((field) => field.name),
    );
    expect(sessionFields).toEqual(new Set(["expiresIn", "tokenType"]));
  });
});
