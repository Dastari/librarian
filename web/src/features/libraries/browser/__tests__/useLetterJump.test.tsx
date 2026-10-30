import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useLetterJump } from "../useLetterJump";

interface Row { id: string; title: string }
const rows: Row[] = Array.from({ length: 200 }, (_, index) => ({ id: `row-${index}`, title: index < 150 ? `Alpha ${index}` : `Zulu ${index}` }));
const getRowId = (row: Row) => row.id;
const getTitle = (row: Row) => row.title;
const roots: HTMLElement[] = [];
const rect = (top: number, height: number): DOMRect => ({ top, bottom: top + height, left: 0, right: 900, width: 900, height, x: 0, y: top, toJSON: () => ({}) });

/** Geometry-only DOM fixture: no private Glass classes or internal row indices. */
function viewport(totalHeight = 12_000) {
  const outer = document.createElement("div");
  const root = document.createElement("div");
  const scroller = document.createElement("div");
  scroller.setAttribute("data-scroll-container", "");
  outer.scrollTop = 71;
  outer.append(root);
  root.append(scroller);
  document.body.append(outer);
  roots.push(outer);
  Object.defineProperties(scroller, {
    clientHeight: { value: 240 }, scrollHeight: { value: totalHeight }, clientTop: { value: 3 },
  });
  scroller.getBoundingClientRect = () => rect(200, 246);
  const row = (item: Row, index: number, top: number, height = 48, gallery = false) => {
    const element = document.createElement("div");
    element.dataset.rowId = item.id;
    if (gallery) {
      const link = document.createElement("a");
      link.href = `/items/${index}`;
      link.textContent = item.title;
      element.append(link);
    } else {
      element.setAttribute("role", "row");
      // Deliberately differs from Glass's current header offset. The hook must
      // derive this from identity + ARIA, rather than hard-code an index base.
      element.setAttribute("aria-rowindex", String(index + 17));
      element.tabIndex = -1;
    }
    element.getBoundingClientRect = () => rect(203 + top - scroller.scrollTop, height);
    scroller.append(element);
    return element;
  };
  return { outer, root, scroller, row };
}

function mount(options: Partial<Parameters<typeof useLetterJump<Row>>[0]> = {}) {
  return renderHook((props) => useLetterJump(props), {
    initialProps: { rows, getRowId, getTitle, hasMore: false, loadMore: async () => false, queryKey: "movies", ...options },
  });
}

async function advance(ms = 16) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms); });
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => window.setTimeout(() => callback(performance.now()), 16));
  vi.stubGlobal("cancelAnimationFrame", (handle: number) => window.clearTimeout(handle));
});
afterEach(() => {
  cleanup();
  roots.splice(0).forEach((root) => root.remove());
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("useLetterJump", () => {
  it("keeps a virtualized later-letter jump pending until it mounts and focuses, using measured fixed row pitch", async () => {
    const view = viewport();
    view.scroller.scrollTop = 4800;
    // A retained focused row can be far from the visible window.
    view.row(rows[0]!, 0, 0);
    view.row(rows[100]!, 100, 4800);
    view.row(rows[101]!, 101, 4848);
    const { result } = mount();
    result.current.containerRef.current = view.root;
    act(() => result.current.jump("Z"));
    await advance();
    expect(view.scroller.scrollTop).toBe(7200);
    expect(result.current.pending).toBe(true);
    const target = view.row(rows[150]!, 150, 7200);
    await advance();
    expect(target).toHaveFocus();
    expect(result.current.pending).toBe(false);
    expect(view.outer.scrollTop).toBe(71);
  });

  it("corrects an overshoot with newly rendered ARIA/geometry samples", async () => {
    const view = viewport();
    const varied = rows.map((row, index) => ({ ...row, title: index < 80 ? "Alpha" : "Zulu" }));
    view.row(varied[0]!, 0, 0, 40);
    view.row(varied[1]!, 1, 40, 40);
    const { result } = mount({ rows: varied });
    result.current.containerRef.current = view.root;
    act(() => result.current.jump("Z"));
    await advance();
    expect(view.scroller.scrollTop).toBe(3200);
    // The first estimate passed the target. Keep row 0 as a focused row, as
    // Glass does, and expose the actual new window after scrolling.
    view.row(varied[90]!, 90, 3200, 40);
    await advance();
    expect(view.scroller.scrollTop).toBeGreaterThan(2800);
    expect(view.scroller.scrollTop).toBeLessThan(3200);
    view.row(varied[79]!, 79, 2960, 40);
    view.row(varied[81]!, 81, 3040, 40);
    await advance();
    expect(view.scroller.scrollTop).toBe(3000);
    expect(result.current.pending).toBe(true);
    const target = view.row(varied[80]!, 80, 3000, 40);
    await advance();
    expect(target).toHaveFocus();
    expect(result.current.pending).toBe(false);
  });

  it.each([false, true])("uses actual variable card geometry and scoped identity (gallery=%s)", async (gallery) => {
    const view = viewport();
    const varied = [{ id: "alpha", title: "Alpha" }, { id: 'z"] duplicate', title: "Zulu" }];
    view.row(varied[0]!, 0, 0, 143, gallery);
    const target = view.row(varied[1]!, 1, 1673, 291, gallery);
    const duplicate = target.cloneNode(true) as HTMLElement;
    document.body.prepend(duplicate);
    roots.push(duplicate);
    const { result } = mount({ rows: varied });
    result.current.containerRef.current = view.root;
    act(() => result.current.jump("Z"));
    await advance();
    expect(view.scroller.scrollTop).toBe(1673);
    expect(gallery ? target.querySelector("a") : target).toHaveFocus();
    expect(duplicate).not.toHaveFocus();
    expect(view.outer.scrollTop).toBe(71);
    expect(result.current.pending).toBe(false);
  });

  it("waits for ascending title sort and in-flight fetching before focusing", async () => {
    const view = viewport();
    const target = view.row(rows[150]!, 150, 7200);
    const ensureTitleSort = vi.fn();
    const loadMore = vi.fn(async () => false);
    const { result, rerender } = mount({ isTitleSorted: false, loading: false, ensureTitleSort, loadMore });
    result.current.containerRef.current = view.root;
    act(() => result.current.jump("Z"));
    expect(ensureTitleSort).toHaveBeenCalledOnce();
    await advance();
    expect(target).not.toHaveFocus();
    rerender({ rows, getRowId, getTitle, hasMore: false, loadMore, queryKey: "movies", isTitleSorted: true, loading: true, ensureTitleSort });
    await advance(50);
    expect(target).not.toHaveFocus();
    expect(result.current.pending).toBe(true);
    rerender({ rows, getRowId, getTitle, hasMore: false, loadMore, queryKey: "movies", isTitleSorted: true, loading: false, ensureTitleSort });
    await advance(50);
    expect(target).toHaveFocus();
    expect(loadMore).not.toHaveBeenCalled();
  });

  it("loads every required page before falling back to the last row of a complete list", async () => {
    const view = viewport();
    const pages = [{ id: "a", title: "Alpha" }, { id: "b", title: "Beta" }, { id: "c", title: "Charlie" }, { id: "d", title: "Delta" }, { id: "e", title: "Echo" }];
    const loadMore = vi.fn(async (): Promise<boolean> => {
      count++;
      rerender({ rows: pages.slice(0, count), getRowId, getTitle, hasMore: count < pages.length, loadMore, queryKey: "movies" });
      return true;
    });
    let count = 1;
    const { result, rerender } = mount({ rows: pages.slice(0, count), hasMore: true, loadMore });
    result.current.containerRef.current = view.root;
    const last = view.row(pages[4]!, 4, 432);
    act(() => result.current.jump("Z"));
    // End each act between pages so React commits the new rows/hasMore and
    // updates the hook's latest-options effect before the next loader turn.
    await advance();
    expect(loadMore).toHaveBeenCalledTimes(1);
    for (let page = 2; page < pages.length; page++) {
      await advance(50);
      expect(loadMore).toHaveBeenCalledTimes(page);
      expect(last).not.toHaveFocus();
      expect(result.current.pending).toBe(true);
    }
    await advance(50);
    expect(loadMore).toHaveBeenCalledTimes(4);
    expect(last).toHaveFocus();
    expect(view.scroller.scrollTop).toBe(432);
    expect(result.current.pending).toBe(false);
  });

  it("cancels a replaced query immediately and ignores its pending page failure during a new jump", async () => {
    const view = viewport();
    let rejectPage!: (reason: Error) => void;
    const loadMore = vi.fn(() => new Promise<boolean>((_resolve, reject) => { rejectPage = reject; }));
    const { result, rerender } = mount({ rows: rows.slice(0, 2), hasMore: true, loadMore });
    result.current.containerRef.current = view.root;
    act(() => result.current.jump("Z"));
    await advance();
    expect(loadMore).toHaveBeenCalledOnce();
    rerender({ rows, getRowId, getTitle, hasMore: false, loadMore, queryKey: "replacement" });
    expect(result.current.pending).toBe(false);
    const target = view.row(rows[150]!, 150, 7200);
    act(() => result.current.jump("Z"));
    await act(async () => { rejectPage(new Error("Old page failed")); });
    expect(result.current.pending).toBe(true);
    await advance();
    expect(target).toHaveFocus();
    expect(result.current.pending).toBe(false);
  });

  it("replaces an unmounted jump and bounds retries when no target can mount", async () => {
    const view = viewport();
    const first = view.row(rows[0]!, 0, 0);
    const { result } = mount();
    result.current.containerRef.current = view.root;
    act(() => result.current.jump("Z"));
    await advance();
    act(() => result.current.jump("A"));
    await advance();
    expect(first).toHaveFocus();
    const staleTarget = view.row(rows[150]!, 150, 7200);
    await advance(100);
    expect(staleTarget).not.toHaveFocus();
    staleTarget.remove();
    act(() => result.current.jump("Z"));
    await advance(2000);
    expect(result.current.pending).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });
});
