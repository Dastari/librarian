import { useCallback, useEffect, useRef, useState } from "react";

import { firstLetter } from "@/lib/utils";

interface UseLetterJumpOptions<TRow> {
  rows: TRow[];
  getRowId: (row: TRow) => string;
  getTitle: (row: TRow) => string;
  hasMore: boolean;
  loadMore: () => Promise<boolean>;
  loading?: boolean;
  /** Filter/scope/view identity, excluding the title sort requested by this hook. */
  queryKey?: string;
  /** False until the requested ascending title sort has been applied. */
  isTitleSorted?: boolean;
  /** Called before jumping when the list is not sorted by title. */
  ensureTitleSort?: () => void;
}

const MAX_SCROLL_ATTEMPTS = 120;
const JUMP_TIMEOUT_MS = 60_000;
const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), [tabindex]';

function rowTop(element: HTMLElement, scroller: HTMLElement) {
  return element.getBoundingClientRect().top - scroller.getBoundingClientRect().top - scroller.clientTop + scroller.scrollTop;
}

/** Keep native scrolling inside this browser's bounded viewport. */
function scrollAndFocus(element: HTMLElement, root: HTMLElement) {
  const scroller = element.closest<HTMLElement>("[data-scroll-container]");
  if (!scroller || !root.contains(scroller) || !scroller.clientHeight || !element.getBoundingClientRect().height) return false;
  scroller.scrollTop = Math.max(0, Math.min(rowTop(element, scroller), scroller.scrollHeight - scroller.clientHeight));
  const focus = element.matches(FOCUSABLE) ? element : element.querySelector<HTMLElement>(FOCUSABLE) ?? element;
  if (focus === element && !element.hasAttribute("tabindex")) element.tabIndex = -1;
  focus.focus({ preventScroll: true });
  return root.ownerDocument.activeElement === focus;
}

/**
 * Estimate from public row semantics and measured positions, then remeasure on the
 * next frame. Remember bracketing samples so an overshoot (e.g. expanded rows)
 * converges instead of repeatedly assuming every row has the same height.
 */
function revealVirtualRow<TRow>(root: HTMLElement, rows: TRow[], getRowId: (row: TRow) => string, targetIndex: number, samples: Map<number, number>) {
  const indexes = new Map(rows.map((row, index) => [getRowId(row), index]));
  for (const scroller of root.querySelectorAll<HTMLElement>("[data-scroll-container]")) {
    const rendered = Array.from(scroller.querySelectorAll<HTMLElement>('[role="row"][aria-rowindex][data-row-id]'))
      .filter((row) => row.closest("[data-scroll-container]") === scroller && row.getBoundingClientRect().height > 0);
    const anchor = rendered.find((row) => indexes.has(row.dataset.rowId!));
    if (!anchor || !scroller.clientHeight) continue;
    const firstAriaIndex = Number(anchor.getAttribute("aria-rowindex")) - indexes.get(anchor.dataset.rowId!)!;
    const targetAriaIndex = targetIndex + firstAriaIndex;
    for (const row of rendered) {
      const index = Number(row.getAttribute("aria-rowindex"));
      if (Number.isFinite(index)) samples.set(index, rowTop(row, scroller));
    }
    const ordered = Array.from(samples).sort(([a], [b]) => a - b);
    const lower = ordered.filter(([index]) => index <= targetAriaIndex).at(-1);
    const upper = ordered.find(([index]) => index >= targetAriaIndex);
    let top: number;
    if (lower && upper && lower[0] !== upper[0]) {
      top = lower[1] + (upper[1] - lower[1]) * (targetAriaIndex - lower[0]) / (upper[0] - lower[0]);
    } else {
      const nearest = lower ?? upper;
      if (!nearest) continue;
      const neighbour = ordered.filter(([index]) => index !== nearest[0]).sort(([a], [b]) => Math.abs(a - nearest[0]) - Math.abs(b - nearest[0]))[0];
      const pitch = neighbour ? (neighbour[1] - nearest[1]) / (neighbour[0] - nearest[0]) : anchor.getBoundingClientRect().height;
      if (!(pitch > 0)) continue;
      top = nearest[1] + (targetAriaIndex - nearest[0]) * pitch;
    }
    scroller.scrollTop = Math.max(0, Math.min(top, scroller.scrollHeight - scroller.clientHeight));
    return;
  }
}

/**
 * Jump in an alphabetically sorted infinite list. Attach containerRef to the
 * browser root; only its marked scroll viewport is moved. Pending lasts until a
 * mounted row is focused, or the request fails, expires, or is replaced.
 */
export function useLetterJump<TRow>(options: UseLetterJumpOptions<TRow>) {
  const containerRef = useRef<HTMLDivElement>(null);
  const latest = useRef(options);
  const generation = useRef(0);
  const [request, setRequest] = useState<{ letter: string; key?: string; generation: number } | null>(null);
  const [active, setActive] = useState<string | null>(null);
  // Update before the request effect, without restarting its bounded retry budget
  // whenever a consumer recreates getTitle/ensureTitleSort during rendering.
  useEffect(() => { latest.current = options; });
  useEffect(() => {
    if (request && request.key !== options.queryKey) {
      generation.current++;
      setRequest(null);
    }
  }, [options.queryKey, request]);

  useEffect(() => {
    if (!request) return;
    let cancelled = false;
    let cancelWait: (() => void) | undefined;
    const current = () => !cancelled && generation.current === request.generation && latest.current.queryKey === request.key;
    const finish = () => {
      if (!cancelled && generation.current === request.generation) setRequest(null);
      cancelled = true;
      cancelWait?.();
    };
    const timeout = window.setTimeout(finish, JUMP_TIMEOUT_MS);
    const wait = (frame: boolean) => new Promise<void>((resolve) => {
      const done = () => { cancelWait = undefined; resolve(); };
      const handle = frame ? requestAnimationFrame(done) : window.setTimeout(done, 50);
      cancelWait = () => {
        if (frame) cancelAnimationFrame(handle);
        else window.clearTimeout(handle);
        done();
      };
    });

    void (async () => {
      let attempts = 0;
      let stalledLoads = 0;
      let sorted = false;
      let sampledRows: TRow[] | undefined;
      const samples = new Map<number, number>();
      try {
        // Let the sort change and newly loaded rows commit before reading DOM.
        await wait(true);
        while (current()) {
          const { rows, getRowId, getTitle, hasMore, loading, isTitleSorted = true, loadMore } = latest.current;
          if (!isTitleSorted) {
            if (sorted) break; // A later manual sort replaces this jump.
            await wait(false);
            continue;
          }
          sorted = true;
          if (loading) {
            await wait(false);
            continue;
          }
          const found = rows.findIndex((row) => {
            const letter = firstLetter(getTitle(row));
            return request.letter === "#" ? letter === "#" : letter >= request.letter;
          });
          const index = found >= 0 ? found : !hasMore ? rows.length - 1 : -1;
          if (index >= 0) {
            const root = containerRef.current;
            if (root) {
              const id = getRowId(rows[index]!);
              // Compare values instead of interpolating arbitrary IDs into CSS.
              const element = Array.from(root.querySelectorAll<HTMLElement>("[data-row-id]"))
                .find((row) => row.dataset.rowId === id);
              if (element && scrollAndFocus(element, root)) break;
              if (sampledRows !== rows) { samples.clear(); sampledRows = rows; }
              revealVirtualRow(root, rows, getRowId, index, samples);
            }
            if (++attempts >= MAX_SCROLL_ATTEMPTS) break;
            await wait(true);
          } else if (hasMore) {
            const loaded = await loadMore();
            if (!current()) break;
            // False can mean a concurrent loader owns the next page. Allow it
            // to settle, but stop a failed/no-progress loader from spinning.
            stalledLoads = loaded ? 0 : stalledLoads + 1;
            if (stalledLoads >= 3) break;
            await wait(false);
          } else break;
        }
      } catch {
        // A failed page ends this request; the browser retains its retry UI.
      } finally {
        window.clearTimeout(timeout);
        finish();
      }
    })();
    return () => {
      cancelled = true;
      window.clearTimeout(timeout);
      cancelWait?.();
    };
  }, [request]);

  const jump = useCallback((letter: string | null) => {
    if (!letter) return;
    latest.current.ensureTitleSort?.();
    setActive(letter);
    setRequest({ letter, key: latest.current.queryKey, generation: ++generation.current });
  }, []);

  return { jump, active, pending: request !== null, containerRef };
}
