import type { CodegenConfig } from "@graphql-codegen/cli";

/**
 * Codegen reads the committed SDL snapshot so it works offline and in CI.
 * Refresh the snapshot with `pnpm schema:pull` while the backend is running.
 */
const config: CodegenConfig = {
  schema: "./schema.graphql",
  documents: ["src/graphql/entities/**/*.graphql", "src/graphql/documents/**/*.graphql"],
  ignoreNoDocuments: false,
  generates: {
    "./src/graphql/generated/": {
      preset: "client",
      presetConfig: {
        fragmentMasking: false,
      },
      config: {
        useTypeImports: true,
        enumsAsTypes: true,
        skipTypename: true,
        avoidOptionals: { field: true, inputValue: false, object: false, defaultValue: false },
        scalars: {
          JSON: { input: "unknown", output: "unknown" },
        },
      },
    },
  },
  hooks: {
    afterAllFileWrite: [],
  },
};

export default config;
