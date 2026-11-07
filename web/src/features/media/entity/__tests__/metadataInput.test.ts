import { describe, expect, it } from "vitest";

import { changedFields, yearValue } from "../metadataInput";

describe("changedFields", () => {
  const saved = { title: "The Matrix", sortTitle: "Matrix", genres: ["Action"], wanted: false };

  it("sends only what changed, so an untouched sort title can follow a new title", () => {
    expect(changedFields(saved, { ...saved, title: "The Matrix Reloaded" })).toEqual({ title: "The Matrix Reloaded" });
  });

  it("sends blank text as null", () => {
    expect(changedFields(saved, { ...saved, sortTitle: "  " })).toEqual({ sortTitle: null });
  });

  it("compares lists and switches by value", () => {
    expect(changedFields(saved, { ...saved, genres: ["Action"] })).toEqual({});
    expect(changedFields(saved, { ...saved, genres: ["Action", "Sci-Fi"], wanted: true })).toEqual({ genres: ["Action", "Sci-Fi"], wanted: true });
  });
});

describe("yearValue", () => {
  it("reads a year, and nothing as no year", () => {
    expect(yearValue("1999")).toBe(1999);
    expect(yearValue(" ")).toBeNull();
  });
});
