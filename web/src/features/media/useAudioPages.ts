import type { DocumentNode, OperationVariables } from "@apollo/client";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useLiveEpoch, useLiveInvalidation } from "@/lib/graphql/live/hooks";

interface AudioConnection<T> {
  edges: Array<{ node: T }>;
  pageInfo: { hasNextPage: boolean; totalCount: number | null };
}

/** Complete audio/artist/collection arrays are staged independently of the
 * detail query's first page. A failed refresh never removes later queue rows. */
export function useAudioPages<T extends { id: string }>(
  scope: string,
  firstPage: AudioConnection<T> | undefined,
  loadPage: (offset: number) => Promise<AudioConnection<T>>,
  live?: { document: DocumentNode; variables: OperationVariables; refreshHead: () => Promise<unknown> },
) {
  const epoch = useLiveEpoch();
  const key = `${epoch}:${scope}`;
  const identity = useRef({ key, version: 0 });
  if (identity.current.key !== key) identity.current = { key, version: identity.current.version + 1 };
  const ignoreNextHead = useRef(false);
  const runningFresh = useRef(false);
  const lastFresh = useRef(false);
  const inputs = useRef({ firstPage, loadPage, live });
  inputs.current = { firstPage, loadPage, live };
  const [snapshot, setSnapshot] = useState<{ key: string; rows: T[]; complete: boolean; loading: boolean; error?: unknown }>();

  const run = useCallback(async (fresh: boolean) => {
    const version = ++identity.current.version;
    lastFresh.current = fresh;
    runningFresh.current = fresh;
    const current = () => identity.current.key === key && identity.current.version === version;
    const { firstPage, loadPage, live } = inputs.current;
    if (!firstPage && !fresh) return;
    setSnapshot((previous) => ({ key, rows: previous?.key === key ? previous.rows : firstPage?.edges.map(({ node }) => node) ?? [], complete: false, loading: true }));
    try {
      let page = fresh ? await loadPage(0) : firstPage!;
      const rows = new Map(page.edges.map(({ node }) => [node.id, node]));
      let offset = page.edges.length;
      while (page.pageInfo.hasNextPage) {
        page = await loadPage(offset);
        if (!current()) return;
        const before = rows.size;
        for (const { node } of page.edges) rows.set(node.id, node);
        if (!page.edges.length || (rows.size === before && page.pageInfo.hasNextPage)) throw new Error("Audio paging did not advance");
        offset += page.edges.length;
      }
      if (page.pageInfo.totalCount != null && rows.size < page.pageInfo.totalCount) throw new Error("Audio collection changed while loading");
      // Refresh title/library metadata too. Head results never own the staged
      // complete array; late pages stay readable throughout this network read.
      if (fresh && live) {
        ignoreNextHead.current = true;
        await live.refreshHead();
      }
      if (current()) setSnapshot({ key, rows: [...rows.values()], complete: true, loading: false });
    } catch (error) {
      ignoreNextHead.current = false;
      if (current()) setSnapshot((previous) => ({ key, rows: previous?.key === key ? previous.rows : [], complete: false, loading: false, error }));
      throw error;
    } finally {
      if (current()) runningFresh.current = false;
    }
  }, [key]);
  useEffect(() => () => { identity.current.version++; }, [key]);
  useEffect(() => {
    if (!firstPage) return;
    if (ignoreNextHead.current) { ignoreNextHead.current = false; return; }
    if (runningFresh.current) return;
    void run(false).catch(() => {});
  }, [key, firstPage, loadPage, run]);
  const refresh = useCallback(() => run(true), [run]);
  // Callers without a detail document are test/local consumers. Production
  // owners always provide the generated operation and explicit refresh callback.
  useLiveInvalidation(live?.document, live?.variables, refresh, { enabled: Boolean(live && scope), key: `audio:${key}`, shared: false });
  const current = snapshot?.key === key ? snapshot : undefined;
  const rows = useMemo(() => {
    const head = new Map(firstPage?.edges.map(({ node }) => [node.id, node]));
    return (current?.rows ?? firstPage?.edges.map(({ node }) => node) ?? []).map((row) => head.get(row.id) ?? row);
  }, [current, firstPage]);
  return {
    rows, complete: Boolean(current?.complete), loading: Boolean(firstPage && (!current || current.loading)), error: current?.error,
    retry: useCallback(() => { void run(lastFresh.current).catch(() => {}); }, [run]),
    updateRow: useCallback((row: T) => setSnapshot((previous) => previous?.key === key ? { ...previous, rows: previous.rows.map((item) => item.id === row.id ? row : item) } : previous), [key]),
  };
}
