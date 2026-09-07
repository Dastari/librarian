// @vitest-environment node
import { describe, expect, it } from "vitest";

import { asBool, asList, asNumber, normalize } from "../useAppSettings";

describe("app setting values", () => {
  it("strips JSON quoting and treats null as empty", () => {
    expect(normalize('"/data/downloads"')).toBe("/data/downloads");
    expect(normalize("/data/downloads")).toBe("/data/downloads");
    expect(normalize("  null  ")).toBe("");
    expect(normalize('"unterminated')).toBe('"unterminated');
    expect(normalize('"broken\\"')).toBe('broken\\');
  });

  it("reads booleans with a fallback", () => {
    expect(asBool("true")).toBe(true);
    expect(asBool("false")).toBe(false);
    expect(asBool(undefined, true)).toBe(true);
    expect(asBool("", true)).toBe(true);
    expect(asBool("yes")).toBe(false);
  });

  it("reads numbers with a fallback", () => {
    expect(asNumber("6881", 0)).toBe(6881);
    expect(asNumber("1.5", 0)).toBe(1.5);
    expect(asNumber(undefined, 6881)).toBe(6881);
    expect(asNumber("", 5)).toBe(5);
    expect(asNumber("many", 5)).toBe(5);
  });

  it("reads a list from JSON or a comma list", () => {
    expect(asList('["a","b"]')).toEqual(["a", "b"]);
    expect(asList("a, b ,c")).toEqual(["a", "b", "c"]);
    expect(asList("[1,2]")).toEqual(["1", "2"]);
    expect(asList("[broken")).toEqual([]);
    expect(asList(undefined)).toEqual([]);
    expect(asList("")).toEqual([]);
  });
});
