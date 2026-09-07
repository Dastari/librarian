import { describe, expect, it } from "vitest";

import { albumCover, artistImage, audiobookCover, collectionBackdrop, collectionPoster, episodeThumb, movieBackdrop, moviePoster, personProfile, showBackdrop, showPoster } from "../artwork";

describe("artwork resolution", () => {
  it("serves movies, episodes and collections from the artwork cache", () => {
    expect(moviePoster("mv-1")).toBe("/api/artwork/movie/mv-1/poster");
    expect(movieBackdrop("mv-1")).toBe("/api/artwork/movie/mv-1/backdrop");
    expect(episodeThumb("ep-1")).toBe("/api/artwork/episode/ep-1/thumbnail");
    expect(collectionPoster({ tmdbCollectionId: 10 })).toBe("/api/artwork/collection/10/poster");
    expect(collectionBackdrop({ tmdbCollectionId: 10 })).toBe("/api/artwork/collection/10/backdrop");
  });

  it("prefers the provider URL carried on the entity", () => {
    expect(showPoster({ id: "s1", posterUrl: "https://cdn/poster.jpg" })).toBe("https://cdn/poster.jpg");
    expect(showBackdrop({ id: "s1", backdropUrl: "https://cdn/back.jpg" })).toBe("https://cdn/back.jpg");
    expect(albumCover({ id: "a1", coverUrl: "https://cdn/cover.jpg" })).toBe("https://cdn/cover.jpg");
    expect(artistImage({ id: "ar1", imageUrl: "https://cdn/artist.jpg" })).toBe("https://cdn/artist.jpg");
    expect(audiobookCover({ id: "b1", coverUrl: "https://cdn/book.jpg" })).toBe("https://cdn/book.jpg");
    expect(collectionPoster({ tmdbCollectionId: 10, posterUrl: "https://cdn/c.jpg" })).toBe("https://cdn/c.jpg");
  });

  it("falls back to the cache when the entity has no URL", () => {
    expect(showPoster({ id: "s1" })).toBe("/api/artwork/show/s1/poster");
    expect(showPoster({ id: "s1", posterUrl: null })).toBe("/api/artwork/show/s1/poster");
    expect(showBackdrop({ id: "s1", backdropUrl: null })).toBe("/api/artwork/show/s1/backdrop");
    expect(albumCover({ id: "a1", coverUrl: null })).toBe("/api/artwork/album/a1/cover");
    expect(artistImage({ id: "ar1", imageUrl: null })).toBe("/api/artwork/artist/ar1/poster");
    expect(audiobookCover({ id: "b1", coverUrl: null })).toBe("/api/artwork/audiobook/b1/cover");
  });

  it("escapes ids that would break the path", () => {
    expect(moviePoster("a/b?c")).toBe("/api/artwork/movie/a%2Fb%3Fc/poster");
  });

  it("has no fallback for people", () => {
    expect(personProfile({ profileUrl: "https://cdn/p.jpg" })).toBe("https://cdn/p.jpg");
    expect(personProfile({ profileUrl: null })).toBeUndefined();
  });
});
