#!/usr/bin/env node
/**
 * After codegen, fix the TypedDocumentNode import to be type-only for verbatimModuleSyntax.
 */
import { existsSync, readFileSync, writeFileSync } from "fs";
import { fileURLToPath } from "url";
import { dirname, join } from "path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const generatedFiles = [
  join(__dirname, "../src/lib/graphql/generated/graphql.ts"),
  join(__dirname, "../src/lib/graphql/generated/types.ts"),
];

for (const generatedPath of generatedFiles) {
  if (!existsSync(generatedPath)) continue;

  let content = readFileSync(generatedPath, "utf8");

  // Ensure TypedDocumentNode is a type-only import (erased at runtime) so we don't
  // require @apollo/client to export TypedDocumentNode at runtime.
  content = content.replace(
    /^import (?:type )?\{ (TypedDocumentNode as DocumentNode) \} from ["']([^"']+)["'];?/m,
    'import type { TypedDocumentNode as DocumentNode } from "$2";',
  );

  // The current codegen stack can emit a second schema/input type block before
  // operation types. Keep the canonical schema block from the TypeScript plugin
  // and drop the duplicate block to avoid duplicate identifiers.
  const operationStart = content.search(
    /\nexport type [A-Za-z0-9_]+(?:Query|Mutation|Subscription)Variables\s*=/,
  );
  if (operationStart !== -1) {
    const firstInput = content.indexOf("\nexport type AddAlbumInput =");
    const secondInput =
      firstInput === -1
        ? -1
        : content.indexOf("\nexport type AddAlbumInput =", firstInput + 1);
    if (secondInput !== -1 && secondInput < operationStart) {
      content = content.slice(0, secondInput) + content.slice(operationStart);
    }
  }

  writeFileSync(generatedPath, content);
}
