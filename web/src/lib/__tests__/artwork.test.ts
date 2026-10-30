import { describe, expect, it } from "vitest";

import { sizedArtwork } from "../api/urls";
import { albumCover, artistImage, audiobookCover, collectionBackdrop, collectionPoster, episodeThumb, movieBackdrop, moviePoster, personProfile, seasonPoster, showBackdrop, showPoster } from "../artwork";

describe("artwork resolution", () => {
  it("serves every library entity from the local artwork cache", () => {
    expect(moviePoster("mv-1")).toBe("/api/artwork/movie/mv-1/poster");
    expect(movieBackdrop("mv-1")).toBe("/api/artwork/movie/mv-1/backdrop");
    expect(showPoster({ id: "s1" })).toBe("/api/artwork/show/s1/poster");
    expect(showBackdrop({ id: "s1" })).toBe("/api/artwork/show/s1/backdrop");
    expect(episodeThumb("ep-1")).toBe("/api/artwork/episode/ep-1/thumbnail");
    expect(albumCover({ id: "a1" })).toBe("/api/artwork/album/a1/cover");
    expect(artistImage({ id: "ar1" })).toBe("/api/artwork/artist/ar1/poster");
    expect(audiobookCover({ id: "b1" })).toBe("/api/artwork/audiobook/b1/cover");
    expect(collectionPoster({ tmdbCollectionId: 10 })).toBe("/api/artwork/collection/10/poster");
    expect(collectionBackdrop({ tmdbCollectionId: 10 })).toBe("/api/artwork/collection/10/backdrop");
  });

  it("never hotlinks the provider URL an entity carries", () => {
    const show = { id: "s1", posterUrl: "https://static.tvmaze.com/p.jpg", backdropUrl: "https://static.tvmaze.com/b.jpg" };
    expect(showPoster(show)).toBe("/api/artwork/show/s1/poster");
    expect(showBackdrop(show)).toBe("/api/artwork/show/s1/backdrop");
  });

  it("uses a season's own poster when it has one, otherwise the show's", () => {
    expect(seasonPoster({ id: "se1", posterUrl: "https://static.tvmaze.com/s.jpg" }, { id: "s1" })).toBe("/api/artwork/season/se1/poster");
    expect(seasonPoster({ id: "se1", posterUrl: null }, { id: "s1" })).toBe("/api/artwork/show/s1/poster");
    expect(seasonPoster(undefined, { id: "s1" })).toBe("/api/artwork/show/s1/poster");
  });

  it("caches cast portraits and has no image for people without one", () => {
    expect(personProfile({ id: "p1", profileUrl: "https://image.tmdb.org/p.jpg" })).toBe("/api/artwork/person/p1/profile");
    expect(personProfile({ id: "p1", profileUrl: null })).toBeUndefined();
    expect(personProfile(null)).toBeUndefined();
  });

  it("escapes ids that would break the path", () => {
    expect(moviePoster("a/b?c")).toBe("/api/artwork/movie/a%2Fb%3Fc/poster");
  });

  it("asks the cache for a stored size close to the displayed width", () => {
    expect(sizedArtwork("/api/artwork/movie/m1/poster", 300)).toMatch(/^\/api\/artwork\/movie\/m1\/poster\?w=\d+$/);
    expect(sizedArtwork("/api/artwork/movie/m1/poster", undefined)).toBe("/api/artwork/movie/m1/poster");
    // Provider search results are not cached and keep their own URL.
    expect(sizedArtwork("https://image.tmdb.org/t/p/w500/x.jpg", 300)).toBe("https://image.tmdb.org/t/p/w500/x.jpg");
    expect(sizedArtwork(undefined, 300)).toBeUndefined();
  });
});
