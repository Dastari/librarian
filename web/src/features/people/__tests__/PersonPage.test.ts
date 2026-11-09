import { describe, expect, it } from "vitest";

import { calendarDate, groupPersonCredits } from "../PersonPage";

type Credit = Parameters<typeof groupPersonCredits>[0][number];

const base = { kind: "cast", guest: false, department: "Acting", job: null, characterName: null, creditOrder: 0, episodeCount: null, movie: null, show: null, season: null, episode: null, album: null, track: null, audiobook: null } as unknown as Credit;
const credit = (patch: Record<string, unknown>) => ({ ...base, id: Math.random().toString(36), ...patch }) as Credit;

describe("a person's credits", () => {
  it("groups by title and keeps every role", () => {
    const grouped = groupPersonCredits([
      credit({ characterName: "Neo", movie: { id: "m1", title: "The Matrix", year: 1999 } }),
      credit({ kind: "crew", department: "Production", job: "Executive Producer", movie: { id: "m1", title: "The Matrix", year: 1999 } }),
      credit({ characterName: "John Wick", movie: { id: "m2", title: "John Wick", year: 2014 } }),
      credit({ show: { id: "s1", name: "Swedish Dicks", year: 2016 }, characterName: "Ronin", episodeCount: 3 }),
      credit({ guest: true, characterName: "Himself", episode: { id: "e1", season: 2, episode: 5, title: "x", showId: "s2" } }),
      credit({ kind: "cast", department: "Narration", job: "Narrator", audiobook: { id: "b1", title: "Book", authorName: "A" } }),
      credit({ kind: "cast", department: "Performance", job: "Bass", track: { id: "t1", title: "Song", albumId: "a1", trackNumber: 1 } }),
    ]);
    expect(grouped.movies.map((m) => m.title)).toEqual(["John Wick", "The Matrix"]);
    expect(grouped.movies[1]!.roles).toEqual(["Neo", "Executive Producer"]);
    expect(grouped.shows).toHaveLength(2);
    expect(grouped.shows.find((s) => s.key === "s2")!.roles).toEqual(["S02E05 · Himself"]);
    expect(grouped.books[0]!.roles).toEqual(["Narrator"]);
    expect(grouped.albums[0]!.roles).toEqual(["Song · Bass"]);
  });

  it("writes calendar dates without shifting them", () => {
    expect(calendarDate("1964-09-02")).toMatch(/1964/);
    expect(calendarDate("1964-09-02")).toMatch(/2/);
    expect(calendarDate(null)).toBeNull();
  });
});
