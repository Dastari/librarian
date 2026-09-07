import { readdirSync, readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { parse, visit } from "graphql";
import { describe, expect, it } from "vitest";

const root = resolve(__dirname, "../..");

function filesUnder(dir: string, extensions: Set<string>): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const path = resolve(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      out.push(...filesUnder(path, extensions));
      continue;
    }
    if (extensions.has(path.split(".").pop() ?? "")) {
      out.push(path);
    }
  }
  return out.sort();
}

function startsUppercase(name: string): boolean {
  return /^[A-Z]/.test(name);
}

describe("GraphQL document contract", () => {
  it("keeps document fields, args, variables, and input object keys lower camelCase", () => {
    const violations: string[] = [];
    const files = filesUnder(resolve(root, "src/lib/graphql/documents"), new Set(["graphql"]));

    for (const file of files) {
      const ast = parse(readFileSync(file, "utf8"));
      visit(ast, {
        Field(node) {
          if (startsUppercase(node.name.value)) {
            violations.push(`${file}: field ${node.name.value}`);
          }
        },
        Argument(node) {
          if (startsUppercase(node.name.value)) {
            violations.push(`${file}: argument ${node.name.value}`);
          }
        },
        ObjectField(node) {
          if (startsUppercase(node.name.value)) {
            violations.push(`${file}: input field ${node.name.value}`);
          }
        },
        VariableDefinition(node) {
          const name = node.variable.name.value;
          if (startsUppercase(name)) {
            violations.push(`${file}: variable $${name}`);
          }
        },
      });
    }

    expect(violations).toEqual([]);
  });

  it("does not define raw gql documents outside the codegen document directory", () => {
    const violations: string[] = [];
    const files = filesUnder(resolve(root, "src"), new Set(["ts", "tsx"]))
      .filter((file) => !file.includes("/src/lib/graphql/generated/"))
      .filter((file) => !file.endsWith("/src/lib/graphql/client.ts"));

    for (const file of files) {
      const contents = readFileSync(file, "utf8");
      if (/(?:\bgql\s*`|\bgql\s*\()/.test(contents)) {
        violations.push(file);
      }
    }

    expect(violations).toEqual([]);
  });

  it("does not use legacy PascalCase GraphQL orderBy field names in runtime variables", () => {
    const legacyOrderFields = [
      "SortTitle",
      "SortName",
      "Year",
      "Runtime",
      "TrackNumber",
      "ArtistName",
      "DurationSecs",
      "CreatedAt",
    ];
    const files = [
      "src/components/library/LibraryMoviesTab.tsx",
      "src/components/library/LibraryShowsTab.tsx",
      "src/components/library/LibraryTracksTab.tsx",
    ];
    const violations: string[] = [];

    for (const file of files) {
      const contents = readFileSync(resolve(root, file), "utf8");
      for (const field of legacyOrderFields) {
        if (contents.includes(`"${field}"`)) {
          violations.push(`${file}: ${field}`);
        }
      }
    }

    expect(violations).toEqual([]);
  });
});
