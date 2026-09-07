import { useCallback, useEffect, useRef, useState } from "react";

import { firstLetter } from "@/lib/utils";

interface UseLetterJumpOptions<TRow> {
  rows: TRow[];
  getRowId: (row: TRow) => string;
  getTitle: (row: TRow) => string;
  hasMore: boolean;
  loadMore: () => Promise<boolean>;
  /** Called before jumping when the list is not sorted by title. */
  ensureTitleSort?: () => void;
}

/**
 * Jump-to-letter for alphabetically sorted infinite lists. Finds the first loaded row at or
 * after the letter and scrolls it into view; if it is not loaded yet, keeps loading pages until
 * it appears or the list ends.
 */
export function useLetterJump<TRow>({ rows, getRowId, getTitle, hasMore, loadMore, ensureTitleSort }: UseLetterJumpOptions<TRow>) {
  const [target, setTarget] = useState<string | null>(null);
  const [active, setActive] = useState<string | null>(null);
  const loading = useRef(false);

  const scrollToRow = useCallback((id: string) => {
    const element = document.querySelector<HTMLElement>(`[data-row-id="${id}"]`);
    if (!element) return false;
    element.scrollIntoView({ block: "start", behavior: "smooth" });
    element.focus({ preventScroll: true });
    return true;
  }, []);

  useEffect(() => {
    if (!target) return;
    const index = rows.findIndex((row) => {
      const letter = firstLetter(getTitle(row));
      return target === "#" ? letter === "#" : letter >= target;
    });
    if (index >= 0) {
      const row = rows[index]!;
      // Wait one frame so newly loaded rows are in the DOM.
      requestAnimationFrame(() => scrollToRow(getRowId(row)));
      setTarget(null);
      return;
    }
    if (hasMore && !loading.current) {
      loading.current = true;
      void loadMore().finally(() => {
        loading.current = false;
      });
      return;
    }
    if (!hasMore) {
      const last = rows[rows.length - 1];
      if (last) requestAnimationFrame(() => scrollToRow(getRowId(last)));
      setTarget(null);
    }
  }, [target, rows, hasMore, loadMore, getRowId, getTitle, scrollToRow]);

  const jump = useCallback(
    (letter: string | null) => {
      if (!letter) return;
      ensureTitleSort?.();
      setActive(letter);
      setTarget(letter);
    },
    [ensureTitleSort],
  );

  return { jump, active, pending: target !== null };
}
