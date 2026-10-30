#!/usr/bin/env node
/**
 * Generates GraphQL operation documents for every macro-generated entity in the
 * backend schema. The backend exposes the same contract for each entity:
 *
 *   query    <plural>(where, orderBy, page): <Entity>Connection!
 *   query    <single>(id): <Entity>
 *   mutation create<Entity>(input) / update<Entity>(id, input) / delete<Entity>(id)
 *   mutation update<Plural>(where, input) / delete<Plural>(where)
 *   subscription <single>Changed(filter)
 *
 * For each entity this writes `src/graphql/entities/<entity>.graphql` containing a
 * `<Entity>Fields` fragment with every scalar field plus the standard operations.
 * Hand-written documents in `src/graphql/documents` spread these fragments and add
 * relation selections. Schema types and TypedDocumentNodes come from codegen;
 * handwritten selection documents remain validated compositions of that schema.
 */
import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import {
  buildSchema,
  getNamedType,
  isEnumType,
  isListType,
  isNonNullType,
  isObjectType,
  isScalarType,
} from "graphql";

const root = process.cwd();
const args = process.argv.slice(2);
const option = (name, fallback) => args.includes(name) ? args[args.indexOf(name) + 1] : fallback;
const schemaPath = resolve(root, option("--schema", "schema.graphql"));
const outputDir = resolve(root, option("--output", "src/graphql/entities"));
const check = args.includes("--check");
const expectedFiles = new Map();

const schema = buildSchema(readFileSync(schemaPath, "utf8"));
const queryType = schema.getQueryType();
const mutationType = schema.getMutationType();
const subscriptionType = schema.getSubscriptionType();

if (!queryType) {
  throw new Error("Schema is missing its query root");
}

const queryFields = queryType.getFields();
const mutationFields = mutationType?.getFields() ?? {};
const subscriptionFields = subscriptionType?.getFields() ?? {};

const lowerFirst = (value) => value.charAt(0).toLowerCase() + value.slice(1);
const upperFirst = (value) => value.charAt(0).toUpperCase() + value.slice(1);

/** Scalar, enum, and list-of-scalar fields are safe to select without sub-selections. */
function isLeafSelectable(type) {
  const named = getNamedType(type);
  return isScalarType(named) || isEnumType(named);
}

function unwrap(type) {
  let current = type;
  while (isNonNullType(current) || isListType(current)) current = current.ofType;
  return current;
}

/** Entities are discovered from `*Connection` list queries with `where`/`orderBy`/`page` args. */
const entities = [];
for (const [fieldName, field] of Object.entries(queryFields)) {
  const returnType = unwrap(field.type);
  if (!isObjectType(returnType) || !returnType.name.endsWith("Connection")) continue;
  const argNames = new Set(field.args.map((arg) => arg.name));
  if (!argNames.has("where") || !argNames.has("page")) continue;
  // Opt-in keyset roots return the same connection, but are not a second
  // entity. Discover the canonical offset root; generate keysets separately.
  if (getNamedType(field.args.find((arg) => arg.name === "page").type).name !== "PageInput") continue;

  const edgesField = returnType.getFields().edges;
  const edgeType = edgesField ? unwrap(edgesField.type) : undefined;
  const nodeField = edgeType && isObjectType(edgeType) ? edgeType.getFields().node : undefined;
  const entityType = nodeField ? unwrap(nodeField.type) : undefined;
  if (!entityType || !isObjectType(entityType)) continue;

  const entityName = entityType.name;
  const singleField = Object.entries(queryFields).find(
    ([, candidate]) =>
      unwrap(candidate.type) === entityType &&
      candidate.args.length === 1 &&
      candidate.args[0].name === "id",
  );

  const whereArg = field.args.find((arg) => arg.name === "where");
  const orderByArg = field.args.find((arg) => arg.name === "orderBy");

  entities.push({
    name: entityName,
    type: entityType,
    pluralField: fieldName,
    pluralName: upperFirst(fieldName),
    singleField: singleField?.[0],
    whereInput: whereArg ? String(whereArg.type) : undefined,
    orderByInput: orderByArg ? String(orderByArg.type) : undefined,
  });
}

const names = entities.map((entity) => entity.name);
if (new Set(names).size !== names.length) throw new Error("Ambiguous generated entity list roots");
// The authorization matrix deliberately enumerates every public registered
// entity. Missing roots must not disappear silently from schema-driven output.
if (!args.includes("--schema")) {
  const registry = JSON.parse(readFileSync(resolve(root, "../backend/tests/fixtures/authorization-matrix.json"), "utf8"));
  for (const { entity } of registry) {
    if (!names.includes(entity)) throw new Error(`Registered entity ${entity} has no generated list root`);
    if (!subscriptionFields[`${lowerFirst(entity)}Changed`]) throw new Error(`Registered entity ${entity} has no generated change subscription`);
  }
}

function inputTypeName(field, argName) {
  const arg = field?.args.find((candidate) => candidate.name === argName);
  return arg ? String(arg.type) : undefined;
}

function toKebab(value) {
  return value.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();
}

let generatedCount = 0;
for (const entity of entities) {
  const { name, type, pluralField, pluralName, singleField, whereInput, orderByInput } = entity;
  const fields = Object.values(type.getFields()).filter((field) => field.args.length === 0 && isLeafSelectable(field.type));
  if (fields.length === 0) continue;

  const fragmentName = `${name}Fields`;
  const lines = [];
  lines.push(`# Generated by scripts/generate-entity-documents.mjs from schema.graphql. Do not edit.`);
  lines.push("");
  lines.push(`fragment ${fragmentName} on ${name} {`);
  for (const field of fields) lines.push(`  ${field.name}`);
  lines.push("}");
  lines.push("");

  const listVars = [
    whereInput ? `$where: ${whereInput}` : null,
    orderByInput ? `$orderBy: ${orderByInput}` : null,
    "$page: PageInput",
  ].filter(Boolean);
  const listArgs = [
    whereInput ? "where: $where" : null,
    orderByInput ? "orderBy: $orderBy" : null,
    "page: $page",
  ].filter(Boolean);

  lines.push(`query Entity${name}List(${listVars.join(", ")}) {`);
  lines.push(`  ${pluralField}(${listArgs.join(", ")}) {`);
  lines.push(`    edges {`);
  lines.push(`      node {`);
  lines.push(`        ...${fragmentName}`);
  lines.push(`      }`);
  lines.push(`    }`);
  lines.push(`    pageInfo {`);
  lines.push(`      hasNextPage`);
  lines.push(`      hasPreviousPage`);
  lines.push(`      totalCount`);
  lines.push(`    }`);
  lines.push(`  }`);
  lines.push(`}`);
  lines.push("");

  if (singleField) {
    lines.push(`query Entity${name}Get($id: ${inputTypeName(queryFields[singleField], "id")}) {`);
    lines.push(`  ${singleField}(id: $id) {`);
    lines.push(`    ...${fragmentName}`);
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }

  const keyset = Object.values(queryFields).find((field) => unwrap(field.type) === unwrap(queryFields[pluralField].type)
    && field.args.some((arg) => arg.name === "page" && getNamedType(arg.type).name === "KeysetPageInput"));
  if (keyset) {
    const keysetWhere = inputTypeName(keyset, "where");
    const keysetPage = inputTypeName(keyset, "page");
    const variables = [keysetWhere ? `$where: ${keysetWhere}` : null, `$page: ${keysetPage}`].filter(Boolean);
    const arguments_ = [keysetWhere ? "where: $where" : null, "page: $page"].filter(Boolean);
    lines.push(`query Entity${name}Keyset(${variables.join(", ")}) {`);
    lines.push(`  ${keyset.name}(${arguments_.join(", ")}) {`);
    lines.push(`    edges { cursor node { ...${fragmentName} } }`);
    lines.push(`    pageInfo { hasNextPage hasPreviousPage startCursor endCursor totalCount }`);
    lines.push(`  }`);
    lines.push(`}`, "");
  }

  const createField = mutationFields[`create${name}`];
  const createInput = inputTypeName(createField, "input");
  if (createField && createInput) {
    const resultType = unwrap(createField.type);
    const resultFields = isObjectType(resultType) ? resultType.getFields() : {};
    const entityField = Object.values(resultFields).find((field) => unwrap(field.type) === type);
    lines.push(`mutation Entity${name}Create($input: ${createInput}) {`);
    lines.push(`  create${name}(input: $input) {`);
    if (resultFields.success) lines.push(`    success`);
    if (resultFields.error) lines.push(`    error`);
    if (entityField) {
      lines.push(`    ${entityField.name} {`);
      lines.push(`      ...${fragmentName}`);
      lines.push(`    }`);
    }
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }

  const updateField = mutationFields[`update${name}`];
  const updateInput = inputTypeName(updateField, "input");
  if (updateField && updateInput && updateField.args.some((arg) => arg.name === "id")) {
    const resultType = unwrap(updateField.type);
    const resultFields = isObjectType(resultType) ? resultType.getFields() : {};
    const entityField = Object.values(resultFields).find((field) => unwrap(field.type) === type);
    lines.push(`mutation Entity${name}Update($id: String!, $input: ${updateInput}) {`);
    lines.push(`  update${name}(id: $id, input: $input) {`);
    if (resultFields.success) lines.push(`    success`);
    if (resultFields.error) lines.push(`    error`);
    if (entityField) {
      lines.push(`    ${entityField.name} {`);
      lines.push(`      ...${fragmentName}`);
      lines.push(`    }`);
    }
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }

  const bulkUpdateField = mutationFields[`update${pluralName}`];
  if (bulkUpdateField && whereInput && updateInput && bulkUpdateField.args.some((arg) => arg.name === "where")) {
    const resultType = unwrap(bulkUpdateField.type);
    const resultFields = isObjectType(resultType) ? resultType.getFields() : {};
    lines.push(`mutation Entity${name}UpdateMany($where: ${whereInput}, $input: ${updateInput}) {`);
    lines.push(`  update${pluralName}(where: $where, input: $input) {`);
    for (const key of ["success", "error", "affectedCount"]) if (resultFields[key]) lines.push(`    ${key}`);
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }

  const deleteField = mutationFields[`delete${name}`];
  if (deleteField && deleteField.args.some((arg) => arg.name === "id")) {
    const resultType = unwrap(deleteField.type);
    const resultFields = isObjectType(resultType) ? resultType.getFields() : {};
    lines.push(`mutation Entity${name}Delete($id: String!) {`);
    lines.push(`  delete${name}(id: $id) {`);
    for (const key of ["success", "error"]) if (resultFields[key]) lines.push(`    ${key}`);
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }

  const bulkDeleteField = mutationFields[`delete${pluralName}`];
  if (bulkDeleteField && whereInput && bulkDeleteField.args.some((arg) => arg.name === "where")) {
    const resultType = unwrap(bulkDeleteField.type);
    const resultFields = isObjectType(resultType) ? resultType.getFields() : {};
    lines.push(`mutation Entity${name}DeleteMany($where: ${whereInput}) {`);
    lines.push(`  delete${pluralName}(where: $where) {`);
    for (const key of ["success", "error", "affectedCount"]) if (resultFields[key]) lines.push(`    ${key}`);
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }

  const changedField = subscriptionFields[`${lowerFirst(name)}Changed`];
  if (changedField) {
    const eventType = unwrap(changedField.type);
    const eventFields = isObjectType(eventType) ? eventType.getFields() : {};
    const filterInput = inputTypeName(changedField, "filter");
    lines.push(
      filterInput
        ? `subscription Entity${name}Changed($filter: ${filterInput}) {`
        : `subscription Entity${name}Changed {`,
    );
    lines.push(filterInput ? `  ${changedField.name}(filter: $filter) {` : `  ${changedField.name} {`);
    for (const key of ["action", "changeKind", "id", "sourceEntity", "sourceId", "path"]) {
      if (eventFields[key]) lines.push(`    ${key}`);
    }
    // ORM 0.45+ authorizes changed events as invalidations. Full entity
    // fragments can include computed/object-valued fields that are rejected
    // at subscription admission. Fetch rich data with the authorized query.
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }

  expectedFiles.set(`${toKebab(name)}.graphql`, `${lines.join("\n").trimEnd()}\n`);
  generatedCount += 1;
}

const coverage = entities.map(({ name, pluralField, singleField }) => ({
  entity: name, list: pluralField, get: singleField ?? null,
  changed: subscriptionFields[`${lowerFirst(name)}Changed`]?.name ?? null,
}));
expectedFiles.set("coverage.json", `${JSON.stringify(coverage, null, 2)}\n`);
if (check) {
  const actual = readdirSync(outputDir);
  const differences = [...expectedFiles].filter(([file, content]) => !actual.includes(file) || readFileSync(resolve(outputDir, file), "utf8") !== content).map(([file]) => file);
  differences.push(...actual.filter((file) => !expectedFiles.has(file)));
  if (differences.length) throw new Error(`Generated documents differ: ${differences.join(", ")}`);
} else {
  rmSync(outputDir, { recursive: true, force: true });
  mkdirSync(outputDir, { recursive: true });
  for (const [file, content] of expectedFiles) writeFileSync(resolve(outputDir, file), content);
}
const written = expectedFiles.size;
console.log(`Generated ${generatedCount} entity documents (${written} files) in src/graphql/entities`);
