import { expect, it } from "vitest";
import { recentDashboardMedia } from "../../src/lib/dashboard";
it("includes movie-only libraries and sorts mixed media by added time", () => {
  const data = {
    shows: {
      edges: [
        {
          node: {
            id: "show",
            name: "Show",
            posterUrl: null,
            createdAt: "2026-07-29T00:00:00Z",
          },
        },
      ],
    },
    movies: {
      edges: [
        {
          node: {
            id: "movie",
            title: "Movie",
            collectionPosterUrl: "/poster",
            createdAt: "1785390864",
          },
        },
      ],
    },
    albums: { edges: [] },
    audiobooks: { edges: [] },
  };
  expect(recentDashboardMedia(data).map((x) => x.href)).toEqual([
    "/movies/movie",
    "/shows/show",
  ]);
  expect(recentDashboardMedia({ ...data, shows: { edges: [] } })[0].title).toBe(
    "Movie",
  );
  expect(recentDashboardMedia(undefined)).toEqual([]);
});
