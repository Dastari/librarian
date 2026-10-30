// @vitest-environment node

import { readdirSync, readFileSync } from "node:fs";

import { Kind, parse } from "graphql";
import { expect, it } from "vitest";

it("generated subscriptions request invalidation metadata instead of rich entity fragments", () => {
  const directory = new URL("../../../graphql/entities/", import.meta.url);
  const allowed = new Set(["action", "changeKind", "id", "sourceEntity", "sourceId", "path"]);
  let subscriptions = 0;

  for (const file of readdirSync(directory).filter((name) => name.endsWith(".graphql"))) {
    const document = parse(readFileSync(new URL(file, directory), "utf8"));
    for (const definition of document.definitions) {
      if (definition.kind !== Kind.OPERATION_DEFINITION || definition.operation !== "subscription") continue;
      subscriptions += 1;
      for (const root of definition.selectionSet.selections) {
        expect(root.kind, file).toBe(Kind.FIELD);
        if (root.kind !== Kind.FIELD) continue;
        const fields = root.selectionSet?.selections ?? [];
        expect(fields.length, file).toBeGreaterThan(0);
        for (const field of fields) {
          expect(field.kind, file).toBe(Kind.FIELD);
          if (field.kind !== Kind.FIELD) continue;
          expect(allowed.has(field.name.value), `${file}: ${field.name.value}`).toBe(true);
          expect(field.selectionSet, file).toBeUndefined();
        }
      }
    }
  }

  expect(subscriptions).toBeGreaterThan(0);
});
