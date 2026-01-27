import type { CodegenConfig } from '@graphql-codegen/cli';

const schemaUrl =
  process.env.VITE_API_URL != null && process.env.VITE_API_URL !== ''
    ? `${process.env.VITE_API_URL}/graphql`
    : 'http://localhost:3001/graphql';

const config: CodegenConfig = {
  schema: schemaUrl,
  // Only .graphql at root to avoid Babel parse of .ts/.tsx (fixes "Unexpected token (388:0)" in pluck).
  documents: ['src/lib/graphql/documents/**/*.graphql'],
  ignore: ['src/lib/graphql/generated/**', 'src/routeTree.gen.ts'],
  ignoreNoDocuments: true,
  generates: {
    // TypedDocumentNode + operation types from .graphql documents (auth, etc.)
    './src/lib/graphql/generated/graphql.ts': {
      documents: ['src/lib/graphql/documents/**/*.graphql'],
      plugins: ['typescript', 'typescript-operations', 'typed-document-node'],
      config: {
        documentNodeImport: '@apollo/client#TypedDocumentNode',
        namingConvention: { typeNames: 'pascal-case#pascalCase', enumValues: 'keep' },
        skipTypename: true,
        enumsAsConst: true,
        scalars: {
          DateTime: 'string',
          Date: 'string',
          UUID: 'string',
          Int64: 'number',
          JSON: 'Record<string, unknown>',
        },
      },
    },
    // Legacy: TypeScript types from schema. Use .graphql only to avoid Babel parse errors on .ts/.tsx (e.g. (388:0) in pluck).
    './src/lib/graphql/generated/types.ts': {
      documents: ['src/lib/graphql/documents/**/*.graphql'],
      plugins: ['typescript', 'typescript-operations'],
      config: {
        namingConvention: { typeNames: 'pascal-case#pascalCase', enumValues: 'keep' },
        onlyOperationTypes: false,
        skipTypename: true,
        avoidOptionals: false,
        enumsAsConst: true,
        exportFragmentSpreadSubTypes: true,
        scalars: {
          DateTime: 'string',
          Date: 'string',
          UUID: 'string',
          Int64: 'number',
          JSON: 'Record<string, unknown>',
        },
      },
    },
    './src/lib/graphql/generated/schema.json': {
      documents: ['src/lib/graphql/documents/**/*.graphql'],
      plugins: ['introspection'],
      config: { minify: false },
    },
  },
  hooks: {
    afterAllFileWrite: [
      "node scripts/fix-graphql-import.mjs",
      "prettier --write",
    ],
  },
};

export default config;
