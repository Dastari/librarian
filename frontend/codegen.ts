import type { CodegenConfig } from '@graphql-codegen/cli';

const schemaUrl =
  process.env.VITE_API_URL != null && process.env.VITE_API_URL !== ''
    ? `${process.env.VITE_API_URL}/graphql`
    : 'http://localhost:3001/graphql';

const config: CodegenConfig = {
  schema: schemaUrl,
  documents: ['src/**/*.tsx', 'src/**/*.ts', 'src/lib/graphql/documents/**/*.graphql'],
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
    // Legacy: TypeScript types from schema (other documents)
    './src/lib/graphql/generated/types.ts': {
      documents: ['src/**/*.tsx', 'src/**/*.ts'],
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
