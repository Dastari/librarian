import { Button } from "glass-ux/button";
import { ScrollArea } from "glass-ux/scroll-area";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import { cn } from "@/lib/utils";

import { SkeletonCard } from "./Skeletons";

export interface PosterGalleryProps<TRow> {
  /** Names the scroll region ("Movies results"). */
  label: string;
  /** Plural noun for the count line ("movies"). */
  noun: string;
  rows: readonly TRow[];
  getRowId: (row: TRow) => string;
  renderCard: (row: TRow) => ReactNode;
  loading?: boolean;
  refetching?: boolean;
  error?: ReactNode;
  emptyState?: ReactNode;
  totalCount?: number | null;
  /** Server batches: the gallery asks for the next one as its end nears. */
  infinite?: { hasMore: boolean; loadMore: () => void | Promise<unknown>; loadingMore?: boolean };
  /** Changes with every new server query (filters, sort); restarts batch tracking. */
  queryKey: string;
  className?: string;
}

/**
 * A library's posters in one bounded, scrolling grid that loads server batches as it nears
 * its end. Cards carry `data-row-id` so the letter rail can find and focus them.
 */
export function PosterGallery<TRow>({ label, noun, rows, getRowId, renderCard, loading = false, refetching = false, error, emptyState, totalCount, infinite, queryKey, className }: PosterGalleryProps<TRow>) {
  const initial = loading && rows.length === 0;
  return (
    <div data-print-flow className={cn("flex min-h-0 min-w-0 flex-1 flex-col gap-3 overflow-hidden p-(--focus-reach)", className)}>
      <ScrollArea className="min-h-0 flex-1" aria-label={`${label} results`}>
        <div className="p-(--focus-reach)">
          {error ? error : !loading && rows.length === 0 ? emptyState : (
            <div className="poster-grid">
              {initial ? Array.from({ length: 12 }, (_, index) => <SkeletonCard key={index} />) : rows.map((row) => {
                const id = getRowId(row);
                return <div key={id} data-row-id={id} className="scroll-mt-4">{renderCard(row)}</div>;
              })}
            </div>
          )}
          {!error && infinite ? <GalleryTail infinite={infinite} count={rows.length} queryKey={queryKey} refetching={refetching || loading} /> : null}
        </div>
      </ScrollArea>
      <div role="status" className="shrink-0 text-label-sm text-muted">
        {initial ? "Loading…" : `${rows.length.toLocaleString()}${typeof totalCount === "number" && totalCount > rows.length ? ` of ${totalCount.toLocaleString()}` : ""} ${noun}`}
      </div>
    </div>
  );
}

/** The scroll sentinel: one request per batch, a retry when one fails. */
function GalleryTail({ infinite, count, queryKey, refetching }: { infinite: NonNullable<PosterGalleryProps<unknown>["infinite"]>; count: number; queryKey: string; refetching: boolean }) {
  const sentinel = useRef<HTMLDivElement>(null);
  const requested = useRef<string | null>(null);
  const inFlight = useRef(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const batch = JSON.stringify([queryKey, count]);
  const latest = useRef({ infinite, batch, refetching });
  latest.current = { infinite, batch, refetching };
  const run = useCallback(async () => {
    const current = latest.current;
    if (!current.infinite.hasMore || current.infinite.loadingMore || current.refetching || inFlight.current) return;
    inFlight.current = true;
    requested.current = current.batch;
    setFailure(null);
    setPending(true);
    try { await current.infinite.loadMore(); } catch { setFailure(current.batch); }
    finally { inFlight.current = false; setPending(false); }
  }, []);
  const failed = failure === batch;
  useEffect(() => {
    const element = sentinel.current;
    if (!element || !infinite.hasMore || infinite.loadingMore || refetching || pending || failed || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting) && requested.current !== latest.current.batch) void run();
    }, { rootMargin: "600px 0px" });
    observer.observe(element);
    return () => observer.disconnect();
  }, [infinite.hasMore, infinite.loadingMore, refetching, pending, failed, batch, run]);
  return (
    <div ref={sentinel} className="flex min-h-10 items-center justify-center gap-2 text-label-sm text-muted" aria-live="polite">
      {failed ? <>Could not load more<Button size="sm" onClick={() => void run()}>Retry</Button></> : pending || infinite.loadingMore ? "Loading more…" : null}
    </div>
  );
}
