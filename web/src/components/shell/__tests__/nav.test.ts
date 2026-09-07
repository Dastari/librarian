// @vitest-environment node
import { describe, expect, it } from "vitest";

import { PRIMARY_NAV, isNavActive } from "../nav";

describe("primary navigation", () => {
  it("lists the destinations in rail order with unique keys and hrefs", () => {
    expect(PRIMARY_NAV.map((item) => item.key)).toEqual(["home", "libraries", "search", "wanted", "downloads", "activity", "settings"]);
    expect(new Set(PRIMARY_NAV.map((item) => item.href)).size).toBe(PRIMARY_NAV.length);
    for (const item of PRIMARY_NAV) {
      expect(item.href.startsWith("/")).toBe(true);
      expect(item.label).toBeTruthy();
      expect(item.icon).toBeTypeOf("object");
    }
  });

  it("puts everything except Activity on the phone tab bar", () => {
    expect(PRIMARY_NAV.filter((item) => item.mobile).map((item) => item.key)).toEqual(["home", "libraries", "search", "wanted", "downloads", "settings"]);
  });

  it("badges downloads and activity", () => {
    expect(PRIMARY_NAV.filter((item) => item.badge).map((item) => [item.key, item.badge])).toEqual([
      ["downloads", "downloads"],
      ["activity", "notifications"],
    ]);
  });
});

describe("isNavActive", () => {
  it("matches home only exactly", () => {
    expect(isNavActive("/", "/")).toBe(true);
    expect(isNavActive("/", "/libraries")).toBe(false);
  });

  it("matches a section and everything under it", () => {
    expect(isNavActive("/libraries", "/libraries")).toBe(true);
    expect(isNavActive("/libraries", "/libraries/lib-1/movies")).toBe(true);
    expect(isNavActive("/settings", "/settings/quality")).toBe(true);
  });

  it("does not match a sibling that merely shares a prefix", () => {
    expect(isNavActive("/search", "/searchable")).toBe(false);
    expect(isNavActive("/downloads", "/download")).toBe(false);
  });
});
