import { act, render, renderHook, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { readPref, usePref, writePref } from "../prefs";

describe("preference store", () => {
  it("reads and writes JSON under a namespaced key", () => {
    expect(readPref("view", "table")).toBe("table");
    writePref("view", "card");
    expect(window.localStorage.getItem("librarian.pref.view")).toBe('"card"');
    expect(readPref("view", "table")).toBe("card");
    writePref("volume", { level: 0.4, muted: false });
    expect(readPref("volume", null)).toEqual({ level: 0.4, muted: false });
  });

  it("falls back when the stored value is not JSON", () => {
    window.localStorage.setItem("librarian.pref.view", "{not json");
    expect(readPref("view", "table")).toBe("table");
  });

  it("exposes the value through a hook and updates it functionally", () => {
    const { result } = renderHook(() => usePref("count", 1));
    expect(result.current[0]).toBe(1);
    act(() => result.current[1]((previous) => previous + 4));
    expect(result.current[0]).toBe(5);
    expect(readPref("count", 0)).toBe(5);
  });

  it("keeps every subscriber of a key in step", async () => {
    function Reader({ id }: { id: string }) {
      const [value, setValue] = usePref("shared", "off");
      return (
        <button type="button" onClick={() => setValue("on")}>
          {id}:{value}
        </button>
      );
    }
    render(
      <>
        <Reader id="a" />
        <Reader id="b" />
      </>,
    );
    await userEvent.click(screen.getByText("a:off"));
    expect(screen.getByText("a:on")).toBeInTheDocument();
    expect(screen.getByText("b:on")).toBeInTheDocument();
  });

  it("picks up writes from another tab", () => {
    const { result } = renderHook(() => usePref("tabbed", "one"));
    act(() => {
      window.localStorage.setItem("librarian.pref.tabbed", '"two"');
      window.dispatchEvent(new StorageEvent("storage", { key: "librarian.pref.tabbed", newValue: '"two"' }));
    });
    expect(result.current[0]).toBe("two");
  });

  it("ignores a corrupt stored value in the hook", () => {
    window.localStorage.setItem("librarian.pref.broken", "{");
    const { result } = renderHook(() => usePref("broken", "fallback"));
    expect(result.current[0]).toBe("fallback");
  });
});
