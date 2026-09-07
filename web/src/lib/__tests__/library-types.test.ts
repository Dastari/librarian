import { describe, expect, it } from "vitest";

import { LIBRARY_TYPES, LIBRARY_TYPE_OPTIONS, libraryType } from "../library-types";

describe("library types", () => {
  it("maps the backend's lower-case names", () => {
    expect(libraryType("movies")).toBe(LIBRARY_TYPES.movies);
    expect(libraryType("music").aspect).toBe("square");
    expect(libraryType("audiobooks").singular).toBe("Audiobook");
  });

  it("accepts the tv aliases the backend has used", () => {
    for (const value of ["tv", "TV", "shows", "tv_shows"]) expect(libraryType(value)).toBe(LIBRARY_TYPES.tv);
  });

  it("falls back to the generic file library", () => {
    expect(libraryType(undefined)).toBe(LIBRARY_TYPES.other);
    expect(libraryType(null)).toBe(LIBRARY_TYPES.other);
    expect(libraryType("photos")).toBe(LIBRARY_TYPES.other);
    expect(LIBRARY_TYPES.other.label).toBe("Files");
  });

  it("gives every type a tint, an icon and a matching key", () => {
    for (const [key, meta] of Object.entries(LIBRARY_TYPES)) {
      expect(meta.type).toBe(key);
      expect(meta.tint).toContain("text-media-");
      expect(meta.tintVar).toMatch(/^var\(--media-/);
      expect(typeof meta.icon).not.toBe("undefined");
    }
  });

  it("offers every type except the catch-all when creating a library", () => {
    expect(LIBRARY_TYPE_OPTIONS.map((meta) => meta.type)).toEqual(["movies", "tv", "music", "audiobooks"]);
  });
});
