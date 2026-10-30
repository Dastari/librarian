// @vitest-environment node
import { readFileSync } from "node:fs";
import { buildSchema, getOperationAST, getVariableValues, validate } from "graphql";
import { EntityAppLogDeleteManyDocument, EntityAppLogListDocument } from "@/graphql/generated/graphql";
import { describe, expect, it } from "vitest";

import { changeLogTableState, initialLogTableState, logTableVariables } from "../glass-table";

describe("logs Glass table GraphQL boundary", () => {
  it("validates the real generated operations and adapter variables against the current schema", () => {
    const schema = buildSchema(readFileSync("schema.graphql", "utf8"));
    const state = initialLogTableState(50);
    state.filters = [{ id: "level", value: { op: "notIn", values: ["DEBUG"] } }, { id: "target", value: { op: "contains", value: "scanner" } }];
    state.search = "Movie";
    for (const [document, variables] of [
      [EntityAppLogListDocument, logTableVariables(state)],
      [EntityAppLogDeleteManyDocument, { where: { id: { isNull: false } } }],
    ] as const) {
      expect(validate(schema, document)).toEqual([]);
      const operation = getOperationAST(document)!;
      expect(getVariableValues(schema, operation.variableDefinitions ?? [], variables).errors).toBeUndefined();
    }
  });
  it("uses camelCase inputs and bounded first batches with actual sort directions", () => {
    const state = initialLogTableState(50);
    state.pagination.pageIndex = 2;
    state.sorting = [{ id: "level", desc: false }, { id: "timestamp", desc: true }, { id: "target", desc: false }];
    expect(logTableVariables(state)).toEqual({ where: {}, orderBy: [{ level: "ASC" }, { timestamp: "DESC" }, { target: "ASC" }], page: { limit: 50, offset: 0 } });
    expect(logTableVariables(initialLogTableState(1000)).page).toEqual({ limit: 100, offset: 0 });
    expect(logTableVariables(initialLogTableState(Number.NaN)).page).toEqual({ limit: 100, offset: 0 });
  });

  it("combines repeated fields with AND and search with nested OR without changing values", () => {
    const state = initialLogTableState(100);
    state.filters = [
      { id: "level", value: { op: "in", values: ["ERROR", "WARN"] } },
      { id: "message", value: { op: "contains", value: "TMDB" } },
      { id: "message", value: { op: "ne", value: "Ignored" } },
    ];
    state.search = "  MixedCase::Provider  ";
    expect(logTableVariables(state).where).toEqual({ and: [
      { level: { inList: ["ERROR", "WARN"] } },
      { message: { contains: "TMDB" } },
      { message: { ne: "Ignored" } },
      { or: [{ message: { contains: "MixedCase::Provider" } }, { target: { contains: "MixedCase::Provider" } }] },
    ] });
  });

  it("maps exclusion to the generated notIn operator and string equality explicitly", () => {
    const state = initialLogTableState(25);
    state.filters = [{ id: "level", value: { op: "notIn", values: ["DEBUG", "TRACE"] } }, { id: "target", value: { op: "eq", value: "librarian::jobs" } }];
    expect(logTableVariables(state).where).toEqual({ and: [{ level: { notIn: ["DEBUG", "TRACE"] } }, { target: { eq: "librarian::jobs" } }] });
    state.filters = [];
    state.search = "   ";
    expect(logTableVariables(state).where).toEqual({});
  });

  it("rejects capabilities that the pilot does not offer instead of silently ignoring them", () => {
    const state = initialLogTableState(100);
    state.sorting = [{ id: "message", desc: false }];
    expect(() => logTableVariables(state)).toThrow("Unsupported log sort");
    state.sorting = [];
    state.filters = [{ id: "fields", value: { op: "contains", value: "x" } }];
    expect(() => logTableVariables(state)).toThrow("Unsupported log filter");
    state.filters = [{ id: "target", value: { op: "isEmpty" } }];
    expect(() => logTableVariables(state)).toThrow("Unsupported log filter operator");
  });

  it("keeps the table on its head after sorting/filtering/search and ignores paging state", () => {
    const state = initialLogTableState(25);
    state.pagination.pageIndex = 4;
    state.selection = { previous: true };
    for (const change of ["sorting", "filters", "search"] as const) {
      expect(changeLogTableState(state, change)).toMatchObject({ pagination: { pageIndex: 0, pageSize: 25 }, selection: {} });
    }
    expect(changeLogTableState(state, "pagination").pagination.pageIndex).toBe(0);
  });
});
