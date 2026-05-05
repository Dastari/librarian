// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { getFirstLetter, useAlphabetFilter } from "../../src/components/data-table";

interface MediaItem {
  title: string;
}

const getTitle = (item: MediaItem) => item.title;

describe("AlphabetFilter utilities", () => {
  it("normalizes common articles and non-letter titles into filter buckets", () => {
    expect(getFirstLetter("The Matrix")).toBe("M");
    expect(getFirstLetter("A Scanner Darkly")).toBe("S");
    expect(getFirstLetter("An American Tail")).toBe("A");
    expect(getFirstLetter("2001 A Space Odyssey")).toBe("#");
    expect(getFirstLetter("")).toBe("#");
  });

  it("derives available letters and toggles filtered items", () => {
    const items: MediaItem[] = [
      { title: "The Matrix" },
      { title: "Alien" },
      { title: "2001 A Space Odyssey" },
    ];

    const { result } = renderHook(() => useAlphabetFilter(items, getTitle));

    expect([...result.current.availableLetters].sort()).toEqual(["#", "A", "M"]);
    expect(result.current.filteredItems).toEqual(items);

    act(() => result.current.onLetterChange("M"));
    expect(result.current.selectedLetter).toBe("M");
    expect(result.current.filteredItems).toEqual([{ title: "The Matrix" }]);

    act(() => result.current.onLetterChange("M"));
    expect(result.current.selectedLetter).toBeNull();
    expect(result.current.filteredItems).toEqual(items);

    act(() => result.current.onLetterChange("#"));
    expect(result.current.filteredItems).toEqual([{ title: "2001 A Space Odyssey" }]);
  });
});

