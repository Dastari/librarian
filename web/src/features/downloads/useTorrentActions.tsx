import { copyText } from "glass-ux/copy-button";
import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionDefinition, type ActionSet } from "glass-ux/actions";
import { IconCopy, IconDownload, IconExternalLink, IconLink, IconPause, IconPlay, IconRefresh, IconTrash } from "glass-ux/icons";
import { useMemo } from "react";

import { EntityTorrentDeleteDocument, EntityTorrentUpdateDocument, PauseTorrentDocument, ProcessSourceDocument, ResumeTorrentDocument } from "@/graphql/generated/graphql";
import { torrentTarget } from "@/lib/entities/torrent";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import type { DownloadRow } from "./useDownloads";

const paused = (row: DownloadRow) => (row.live?.state ?? row.record.state).toLowerCase() === "paused";
const finished = (row: DownloadRow) => (row.live?.progress ?? row.record.progress) >= 1;
const unavailable = (rows: readonly DownloadRow[]) => rows.some((row) => !row.live) ? "Some selected downloads are not loaded in the torrent client. Deselect them to continue." : false;
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
  const [update] = useMutation(EntityTorrentUpdateDocument);
  const [deleteTorrent] = useMutation(EntityTorrentDeleteDocument);
  const [process] = useMutation(ProcessSourceDocument);

  return useMemo(() => {
    const each = async (rows: readonly DownloadRow[], verb: string, run: (row: DownloadRow) => Promise<unknown>) => {
      const failures: string[] = [];
      try {
        for (const row of rows) {
          try { await run(row); }
          catch (error) { failures.push(`${row.record.name}: ${errorMessage(error)}`); }
        }
      } finally { onChanged(); }
      if (failures.length) throw new Error(`${verb} succeeded for ${rows.length - failures.length} of ${rows.length} downloads. ${failures.length} failed. ${failures.join("; ")}`);
    };
    /**
     * Removal is the generated `deleteTorrent`; its lifecycle hook stops the transfer and, when
     * the stored `deleteFilesOnRemove` flag is set, deletes the files. The flag is therefore set
     * (or cleared) first, and put back if the delete is refused so a later plain "Remove" can
     * never delete files. The row's entity id is used, never the live client's numeric id, so a
     * stored download that is not loaded in the client can still be removed.
     */
    const removeDownload = async (row: DownloadRow, deleteFiles: boolean) => {
      const id = row.record.id;
      const flag = async (value: boolean) => assertSuccess((await update({ variables: { id, input: { deleteFilesOnRemove: value } } })).data?.updateTorrent, "Remove failed");
      if (deleteFiles || row.record.deleteFilesOnRemove) await flag(deleteFiles);
      try {
        assertSuccess((await deleteTorrent({ variables: { id } })).data?.deleteTorrent, "Remove failed");
      } catch (error) {
        if (deleteFiles) await flag(false).catch(() => {});
        throw error;
      }
    };
    const actions: ActionDefinition<DownloadRow>[] = [
      {
        id: "open-target", bulk: true, label: "Open linked item", icon: IconExternalLink, group: "open", priority: 2,
        visible: (rows) => rows.length === 1 && Boolean(torrentTarget(rows[0]!.record)?.href),
        run: (rows, context) => { const href = torrentTarget(rows[0]!.record)?.href; if (href) context.navigate(href); },
      },
    ];
    actions.push({
      id: "copy-names", label: "Copy download names", icon: IconCopy, group: "open", bulk: true,
      run: (rows) => copyText(rows.map((row) => row.record.name).join("\n")), successMessage: () => "Download names copied",
    });
    if (enabled) {
      actions.push(
        {
          id: "pause", label: (rows) => plural(rows, "Pause", "Pause {n} downloads"), icon: IconPause, group: "manage", priority: 1, bulk: true,
          visible: (rows) => rows.some((row) => !paused(row)),
          disabledReason: (rows) => unavailable(rows.filter((row) => !paused(row))),
          run: (rows) => each(rows.filter((row) => !paused(row)), "Pause", async (row) => assertSuccess((await pause({ variables: { id: row.live!.id } })).data?.pauseTorrent, "Pause failed")),
          successMessage: (rows) => plural(rows.filter((row) => !paused(row)), "Paused", "Paused {n} downloads"),
        },
        {
          id: "resume", label: (rows) => plural(rows, "Resume", "Resume {n} downloads"), icon: IconPlay, group: "manage", priority: 1, bulk: true,
          visible: (rows) => rows.some(paused),
          disabledReason: (rows) => unavailable(rows.filter(paused)),
          run: (rows) => each(rows.filter(paused), "Resume", async (row) => assertSuccess((await resume({ variables: { id: row.live!.id } })).data?.resumeTorrent, "Resume failed")),
          successMessage: (rows) => plural(rows.filter(paused), "Resumed", "Resumed {n} downloads"),
        },
        { id: "link", bulk: true, label: "Link to library…", icon: IconLink, group: "organise", priority: 2, visible: (rows) => rows.length === 1, run: (rows) => onLink(rows[0]!) },
        {
          id: "import", label: (rows) => plural(rows, "Import now", "Import {n} downloads"), icon: IconDownload, group: "organise", bulk: true,
          visible: (rows) => rows.every(finished),
          run: (rows) => each(rows, "Import", async (row) => assertSuccess((await process({ variables: { sourceId: row.record.infoHash, sourceType: "torrent" } })).data?.processSource, "Import failed")),
          successMessage: (rows) => plural(rows, "Import started", "Import started for {n} downloads"),
        },
        {
          id: "recheck", bulk: true, label: "Retry import", icon: IconRefresh, group: "organise",
          visible: (rows) => rows.length === 1 && rows[0]!.record.postProcessStatus === "failed",
          run: async (rows) => { assertSuccess((await process({ variables: { sourceId: rows[0]!.record.infoHash, sourceType: "torrent" } })).data?.processSource, "Import failed"); onChanged(); },
          successMessage: () => "Import started",
        },
        {
          id: "remove", label: (rows) => plural(rows, "Remove", "Remove {n} downloads"), icon: IconTrash, group: "danger", bulk: true,
          confirm: (rows) => ({ title: plural(rows, `Remove ${rows[0]!.record.name}?`, "Remove {n} downloads?"), body: "Stops the transfer and forgets the torrent. Downloaded files stay on disk, and files already imported into a library are never affected.", confirmLabel: "Remove", tone: "danger" }),
          run: (rows) => each(rows, "Remove", (row) => removeDownload(row, false)),
          successMessage: (rows) => plural(rows, "Removed", "Removed {n} downloads"),
        },
        {
          id: "remove-files", label: (rows) => plural(rows, "Remove and delete files", "Remove {n} and delete files"), icon: IconTrash, group: "danger", bulk: true,
          confirm: (rows) => ({ title: plural(rows, `Delete ${rows[0]!.record.name}?`, "Delete {n} downloads?"), body: "Stops the transfer and deletes the downloaded files from the download folder. Files already imported into a library are never affected. This cannot be undone.", confirmLabel: "Delete files", tone: "danger" }),
          run: (rows) => each(rows, "Remove", (row) => removeDownload(row, true)),
          successMessage: (rows) => plural(rows, "Removed and deleted", "Removed {n} and deleted their files"),
        },
      );
    }
    return defineActions("download", actions);
  }, [deleteTorrent, enabled, onChanged, onLink, pause, process, resume, update]);
}
