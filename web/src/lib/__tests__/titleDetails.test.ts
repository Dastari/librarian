import { describe, expect, it } from "vitest";

import { alternativeTitles, contentRatings, externalIds, mainTrailer, preferredRating, videos } from "../titleDetails";

describe("title details", () => {
  it("reads ratings and prefers the viewer's country, then the US", () => {
    const ratings = contentRatings(JSON.stringify([{ country: "GB", rating: "15", kind: "theatrical" }, { country: "US", rating: "R" }, { country: "FR" }]));
    expect(ratings).toHaveLength(2);
    expect(preferredRating(ratings, "GB")?.rating).toBe("15");
    expect(preferredRating(ratings, "DE")?.rating).toBe("R");
    expect(preferredRating([], "DE")).toBeUndefined();
  });

  it("keeps only videos with an https link and picks the official trailer", () => {
    const list = videos(JSON.stringify([
      { site: "YouTube", key: "a", url: "https://www.youtube.com/watch?v=a", name: "Fan trailer", kind: "Trailer", official: false },
      { site: "YouTube", key: "b", url: "https://www.youtube.com/watch?v=b", name: "Official Trailer", kind: "Trailer", official: true },
      { site: "Odd", key: "c", url: null, name: "No link", kind: "Trailer", official: true },
      { site: "YouTube", key: "d", url: "javascript:alert(1)", name: "Bad", kind: "Clip", official: true },
    ]));
    expect(list.map((video) => video.key)).toEqual(["a", "b"]);
    expect(mainTrailer(list)?.key).toBe("b");
  });

  it("survives missing and malformed text", () => {
    expect(videos(null)).toEqual([]);
    expect(contentRatings("{not json")).toEqual([]);
    expect(alternativeTitles(JSON.stringify({ title: "not a list" }))).toEqual([]);
    expect(externalIds(JSON.stringify({ imdb: "tt0133093", tvdb: 121361, empty: "" }))).toEqual({ imdb: "tt0133093", tvdb: "121361" });
    expect(externalIds("[]")).toEqual({});
  });
});
