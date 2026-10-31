import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionDefinition, type ActionSet } from "glass-ux/actions";
import { IconDownload, IconExternalLink, IconLink, IconPause, IconPlay, IconRefresh, IconTrash } from "glass-ux/icons";
import { useMemo } from "react";

import { PauseTorrentDocument, ProcessSourceDocument, RemoveTorrentDocument, ResumeTorrentDocument } from "@/graphql/generated/graphql";
import { torrentTarget } from "@/lib/entities/torrent";
import { assertSuccess } from "@/lib/graphql/errors";

import type { DownloadRow } from "./useDownloads";

const NOT_LOADED = "Not loaded in the torrent client";
const paused = (row: DownloadRow) => (row.live?.state ?? row.record.state).toLowerCase() === "paused";
const finished = (row: DownloadRow) => (row.live?.progress ?? row.record.progress) >= 1;
const plural = (rows: readonly DownloadRow[], one: string, many: string) => (rows.length === 1 ? one : many.replace("{n}", String(rows.length)));

interface TorrentActionHandlers {
  /** Opens the library picker (LinkTorrentDialog). */
  onLink: (row: DownloadRow) => void;
  /** Re-reads the list after a change the subscription does not carry. */
  onChanged: () => void;
}

/**
 * A download's actions, defined once for its row menu, right-click, the bulk bar, its detail
 * pane and the command palette (docs/entity-tables.md). Client actions need the torrent to be
 * loaded in the client; a stored record without one says so instead of failing.
 */
export function useTorrentActions({ onLink, onChanged }: TorrentActionHandlers, enabled: boolean): ActionSet<DownloadRow> {
  const [pause] = useMutation(PauseTorrentDocument);
  const [resume] = useMutation(ResumeTorrentDocument);
  const [remove] = useMutation(RemoveTorrentDocument);
  const [process] = useMutation(ProcessSourceDocument);

  return useMemo(() => {
    const each = async (rows: readonly DownloadRow[], run: (row: DownloadRow & { live: NonNullable<DownloadRow["live"]> }) => Promise<unknown>) => {
      for (const row of rows) if (row.live) await run(row as DownloadRow & { live: NonNullable<DownloadRow["live"]> });
      onChanged();
    };
    const actions: ActionDefinition<DownloadRow>[] = [
      {
        id: "open-target", label: "Open linked item", icon: IconExternalLink, group: "open", priority: 2,
        visible: (rows) => rows.length === 1 && Boolean(torrentTarget(rows[0]!.record)?.href),
        run: (rows, context) => { const href = torrentTarget(rows[0]!.record)?.href; if (href) context.navigate(href); },
      },
    ];
    if (enabled) {
      actions.push(
        {
          id: "pause", label: (rows) => plural(rows, "Pause", "Pause {n} downloads"), icon: IconPause, group: "manage", priority: 1, bulk: true,
          visible: (rows) => rows.some((row) => !paused(row)),
          disabledReason: (rows) => (rows.every((row) => !row.live) ? NOT_LOADED : false),
          run: (rows) => each(rows.filter((row) => !paused(row)), async (row) => assertSuccess((await pause({ variables: { id: row.live.id } })).data?.pauseTorrent, "Pause failed")),
          successMessage: (rows) => plural(rows, "Paused", "Paused {n} downloads"),
        },
        {
          id: "resume", label: (rows) => plural(rows, "Resume", "Resume {n} downloads"), icon: IconPlay, group: "manage", priority: 1, bulk: true,
          visible: (rows) => rows.some(paused),
          disabledReason: (rows) => (rows.every((row) => !row.live) ? NOT_LOADED : false),
          run: (rows) => each(rows.filter(paused), async (row) => assertSuccess((await resume({ variables: { id: row.live.id } })).data?.resumeTorrent, "Resume failed")),
          successMessage: (rows) => plural(rows, "Resumed", "Resumed {n} downloads"),
        },
        { id: "link", label: "Link to library…", icon: IconLink, group: "organise", priority: 2, visible: (rows) => rows.length === 1, run: (rows) => onLink(rows[0]!) },
        {
          id: "import", label: (rows) => plural(rows, "Import now", "Import {n} downloads"), icon: IconDownload, group: "organise", bulk: true,
          visible: (rows) => rows.every(finished),
          run: async (rows) => {
            for (const row of rows) assertSuccess((await process({ variables: { sourceId: row.record.id, sourceType: "torrent" } })).data?.processSource, "Import failed");
            onChanged();
          },
          successMessage: (rows) => plural(rows, "Import started", "Import started for {n} downloads"),
        },
        {
          id: "recheck", label: "Retry import", icon: IconRefresh, group: "organise",
          visible: (rows) => rows.length === 1 && rows[0]!.record.postProcessStatus === "failed",
          run: async (rows) => { assertSuccess((await process({ variables: { sourceId: rows[0]!.record.id, sourceType: "torrent" } })).data?.processSource, "Import failed"); onChanged(); },
          successMessage: () => "Import started",
        },
        {
          id: "remove", label: (rows) => plural(rows, "Remove", "Remove {n} downloads"), icon: IconTrash, group: "danger", bulk: true,
          disabledReason: (rows) => (rows.every((row) => !row.live) ? NOT_LOADED : false),
          confirm: (rows) => ({ title: plural(rows, `Remove ${rows[0]!.record.name}?`, "Remove {n} downloads?"), body: "Stops the transfer and forgets the torrent. Downloaded files stay on disk, and files already imported into a library are never affected.", confirmLabel: "Remove", tone: "danger" }),
          run: (rows) => each(rows, async (row) => assertSuccess((await remove({ variables: { id: row.live.id, deleteFiles: false } })).data?.removeTorrent, "Remove failed")),
          successMessage: (rows) => plural(rows, "Removed", "Removed {n} downloads"),
        },
        {
          id: "remove-files", label: (rows) => plural(rows, "Remove and delete files", "Remove {n} and delete files"), icon: IconTrash, group: "danger", bulk: true,
          disabledReason: (rows) => (rows.every((row) => !row.live) ? NOT_LOADED : false),
          confirm: (rows) => ({ title: plural(rows, `Delete ${rows[0]!.record.name}?`, "Delete {n} downloads?"), body: "Stops the transfer and deletes the downloaded files from the download folder. Files already imported into a library are never affected. This cannot be undone.", confirmLabel: "Delete files", tone: "danger" }),
          run: (rows) => each(rows, async (row) => assertSuccess((await remove({ variables: { id: row.live.id, deleteFiles: true } })).data?.removeTorrent, "Remove failed")),
          successMessage: (rows) => plural(rows, "Removed and deleted", "Removed {n} and deleted their files"),
        },
      );
    }
    return defineActions("download", actions);
  }, [enabled, onChanged, onLink, pause, process, remove, resume]);
}
