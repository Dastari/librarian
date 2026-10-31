import { describe, expect, it } from "vitest";

import type { LibrarySearchQuery } from "@/graphql/generated/graphql";

import { fold, matchRank, rankHits, startHits, suggestTerms } from "../searchResults";

const edges = <T,>(nodes: T[]) => ({ edges: nodes.map((node) => ({ node })), pageInfo: { totalCount: nodes.length } });

function search(partial: Partial<Record<"movies" | "shows" | "albums" | "artists" | "audiobooks" | "episodes" | "people", unknown[]>>): LibrarySearchQuery {
  return {
    movies: edges(partial.movies ?? []),
    shows: edges(partial.shows ?? []),
    albums: edges(partial.albums ?? []),
    artists: edges(partial.artists ?? []),
    audiobooks: edges(partial.audiobooks ?? []),
    episodes: edges(partial.episodes ?? []),
    people: { edges: (partial.people ?? []).map((node) => ({ node })) },
  } as unknown as LibrarySearchQuery;
}

describe("matchRank", () => {
  it("orders exact, prefix, word start, then contains", () => {
    expect(matchRank("Star Trek", "star trek")).toBe(0);
    expect(matchRank("Stardust", "star")).toBe(1);
    expect(matchRank("Rogue One: A Star Wars Story", "star")).toBe(2);
    expect(matchRank("Mustard", "star")).toBe(3);
    expect(matchRank("Alien", "star")).toBe(4);
  });

  it("ignores case and accents", () => {
    expect(fold("Amélie")).toBe("amelie");
    expect(matchRank("Amélie", "AME")).toBe(1);
  });
});

describe("rankHits", () => {
  it("puts titles that start with the query ahead of other matches, across kinds", () => {
    const hits = rankHits("st", search({
      movies: [{ id: "m1", title: "Mustang", year: 2015 }, { id: "m2", title: "Star Trek", year: 2009 }],
      shows: [{ id: "s1", name: "Stranger Things", year: 2016 }],
      albums: [{ id: "a1", name: "Stadium Arcadium", year: 2006 }],
    }));
    expect(hits.map((hit) => hit.title)).toEqual(["Star Trek", "Stranger Things", "Stadium Arcadium", "Mustang"]);
    expect(hits[0]).toMatchObject({ kind: "movie", art: "wide", to: "/movies/$movieId", params: { movieId: "m2" } });
    expect(hits[2]).toMatchObject({ kind: "album", art: "square" });
  });

  it("explains films found through a director or a cast member", () => {
    const hits = rankHits("nolan", search({
      movies: [{ id: "m1", title: "Inception", director: "Christopher Nolan" }],
      people: [{ id: "p1", name: "Christopher Nolan" }],
    }), [{ id: "c1", personId: "p1", characterName: null, movie: { id: "m2", title: "Following" } }] as never);
    expect(hits.map((hit) => [hit.title, hit.via])).toEqual([
      ["Following", "With Christopher Nolan"],
      ["Inception", "Directed by Christopher Nolan"],
    ]);
  });

  it("lists a film once even when it matches by title and by cast", () => {
    const hits = rankHits("arrival", search({ movies: [{ id: "m1", title: "Arrival" }], people: [{ id: "p1", name: "Amy Adams" }] }), [{ id: "c1", personId: "p1", characterName: "Louise", movie: { id: "m1", title: "Arrival" } }] as never);
    expect(hits).toHaveLength(1);
    expect(hits[0]!.via).toBeUndefined();
  });
});

describe("suggestTerms", () => {
  it("offers people, then titles, never the query itself", () => {
    const data = search({ movies: [{ id: "m1", title: "Star Trek" }, { id: "m2", title: "st" }], people: [{ id: "p1", name: "Steven Seagal" }] });
    expect(suggestTerms("st", data, rankHits("st", data))).toEqual(["Steven Seagal", "Star Trek"]);
  });

  it("offers nothing without a query", () => {
    expect(suggestTerms(" ", search({}), [])).toEqual([]);
  });
});

describe("startHits", () => {
  it("interleaves the newest films and shows", () => {
    const hits = startHits({ movies: edges([{ id: "m1", title: "A" }, { id: "m2", title: "B" }]), shows: edges([{ id: "s1", name: "C" }]) } as never);
    expect(hits.map((hit) => hit.key)).toEqual(["movie:m1", "show:s1", "movie:m2"]);
  });
});
