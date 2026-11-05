import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { copyText } from "glass-ux/copy-button";
import { IconCopy, IconExternalLink, IconFile, IconGauge, IconInfo, IconLinkOff, IconRefresh, IconSearch, IconSparkle, IconTrash } from "glass-ux/icons";
import { useMemo, useRef, useState, type ReactNode } from "react";

import { ManualMatchDialog } from "@/features/libraries/ManualMatchDialog";
import {
  AnalyzeMediaFileDocument,
  DeleteMediaFileFromDiskDocument,
  EvaluateMediaFileQualityDocument,
  MatchMediaFileDocument,
  OrganizeMediaFileDocument,
  UnmatchMediaFileDocument,
  type LinkedMediaFileFragment,
  type MediaFileFieldsFragment,
} from "@/graphql/generated/graphql";
import { runSelectionAction } from "@/lib/actions/runSelectionAction";
import { useIsAdmin } from "@/lib/auth/useSession";
import { fileName, linkedItem } from "@/lib/entities/linked";

import { closeFileDialog, openFileDialog } from "./fileDialog";

/** A file as any page has it: the linked records come with lists and the dialog, not title pages. */
export type FileEntity = MediaFileFieldsFragment & Partial<LinkedMediaFileFragment>;

export function isFileLinked(file: Pick<FileEntity, "movieId" | "episodeId" | "trackId" | "chapterId">): boolean {
  return Boolean(file.movieId || file.episodeId || file.trackId || file.chapterId);
}

/** What the file is, in one line: 1920x808 · HEVC · AC3. */
export function fileFormat(file: Pick<FileEntity, "resolution" | "videoCodec" | "audioCodec" | "isHdr" | "hdrType">): string {
  return [file.resolution, file.videoCodec?.toUpperCase(), file.audioCodec?.toUpperCase(), file.isHdr ? file.hdrType ?? "HDR" : null].filter(Boolean).join(" · ");
}

/**
 * A file's actions, defined once for its table row, right-click, the bulk bar, its dialog and
 * its panel on a title's page (docs/entity-tables.md). They run the same pipeline mutations the
 * scanner uses. `dialogs` renders what they open; mount it beside whatever uses the actions.
 */
export function useFileActions(onChanged?: () => void, { inDialog = false }: { inDialog?: boolean } = {}): { actions: ActionSet<FileEntity>; dialogs: ReactNode } {
  const isAdmin = useIsAdmin();
  const [matching, setMatching] = useState<FileEntity | null>(null);
  const [matchFile] = useMutation(MatchMediaFileDocument);
  const [analyzeFile] = useMutation(AnalyzeMediaFileDocument);
  const [organizeFile] = useMutation(OrganizeMediaFileDocument);
  const [unmatchFile] = useMutation(UnmatchMediaFileDocument);
  const [evaluate] = useMutation(EvaluateMediaFileQualityDocument);
  const [deleteFile] = useMutation(DeleteMediaFileFromDiskDocument, { refetchQueries: ["LibraryMediaFiles"] });
  const changed = useRef(onChanged);
  changed.current = onChanged;

  const actions = useMemo(() => {
    const run = (rows: readonly FileEntity[], label: string, task: (row: FileEntity) => Promise<{ ok: boolean; message?: string | null }>) =>
      runSelectionAction(rows, label, (row) => fileName(row.path), async (row) => {
        const result = await task(row);
        if (!result.ok) throw new Error(result.message || `${label} did not complete`);
      }, () => { changed.current?.(); });
    return defineActions<FileEntity>("media-file", [
      ...(inDialog ? [] : [{ id: "details", bulk: true, label: "File details", icon: IconInfo, group: "open" as const, priority: 1, visible: (rows: readonly FileEntity[]) => rows.length === 1, run: (rows: readonly FileEntity[]) => openFileDialog(rows[0]!.id) }]),
      {
        id: "open-item", bulk: true, label: "Open linked item", icon: IconExternalLink, group: "open", priority: 2,
        visible: (rows) => rows.length === 1 && Boolean(linkedItem(rows[0]!)?.href),
        run: (rows, context) => { const href = linkedItem(rows[0]!)?.href; if (href) { closeFileDialog(); context.navigate(href); } },
      },
      { id: "copy-paths", label: (rows) => rows.length === 1 ? "Copy file path" : "Copy file paths", icon: IconCopy, group: "open", bulk: true, run: (rows) => copyText(rows.map((row) => row.path).join("\n")), successMessage: () => "Copied" },
      ...(isAdmin ? [
        {
          id: "automatch", label: "Match automatically", icon: IconSparkle, group: "organise" as const, priority: 1, bulk: true,
          visible: (rows: readonly FileEntity[]) => rows.some((row) => !isFileLinked(row)),
          disabledReason: (rows: readonly FileEntity[]) => rows.some(isFileLinked) ? "Some selected files are already linked. Deselect them to match." : false,
          run: (rows: readonly FileEntity[]) => run(rows, "Match", async (row) => {
            const { data } = await matchFile({ variables: { input: { mediaFileId: row.id, libraryId: row.libraryId ?? "", autoMatch: true, allowProviderFallback: true } } });
            return { ok: Boolean(data?.matchMediaFile.success), message: data?.matchMediaFile.reason };
          }), successMessage: () => "Matching complete",
        },
        { id: "manual", bulk: true, label: "Match manually…", icon: IconSearch, group: "organise" as const, priority: 2, visible: (rows: readonly FileEntity[]) => rows.length === 1 && Boolean(rows[0]!.libraryId), run: (rows: readonly FileEntity[]) => setMatching(rows[0]!) },
        {
          id: "analyze", label: (rows: readonly FileEntity[]) => rows.length === 1 ? "Re-analyze" : `Re-analyze ${rows.length} files`, icon: IconRefresh, group: "manage" as const, priority: 2, bulk: true,
          run: (rows: readonly FileEntity[]) => run(rows, "Analysis", async (row) => {
            const { data } = await analyzeFile({ variables: { mediaFileId: row.id, path: row.path } });
            return { ok: Boolean(data?.analyzeMediaFile.success), message: data?.analyzeMediaFile.message };
          }), successMessage: () => "Analysis requested",
        },
        {
          id: "evaluate", label: "Re-evaluate quality", icon: IconGauge, group: "manage" as const, priority: 4, bulk: true,
          run: (rows: readonly FileEntity[]) => run(rows, "Evaluation", async (row) => {
            const { data } = await evaluate({ variables: { mediaFileId: row.id } });
            return { ok: Boolean(data?.evaluateMediaFileQuality.success), message: data?.evaluateMediaFileQuality.error };
          }), successMessage: () => "Quality re-evaluated",
        },
        {
          id: "organize", label: "Organize", icon: IconFile, group: "manage" as const, priority: 3, bulk: true,
          visible: (rows: readonly FileEntity[]) => rows.some(isFileLinked),
          disabledReason: (rows: readonly FileEntity[]) => rows.some((row) => !isFileLinked(row)) ? "Some selected files are unmatched. Match them before organizing." : false,
          run: (rows: readonly FileEntity[]) => run(rows, "Organize", async (row) => {
            const { data } = await organizeFile({ variables: { input: { mediaFileId: row.id } } });
            return { ok: Boolean(data?.organizeMediaFile.success), message: data?.organizeMediaFile.reason };
          }), successMessage: () => "Files organized",
        },
        {
          id: "unmatch", label: "Unlink", icon: IconLinkOff, group: "danger" as const, bulk: true,
          visible: (rows: readonly FileEntity[]) => rows.some(isFileLinked),
          disabledReason: (rows: readonly FileEntity[]) => rows.some((row) => !isFileLinked(row)) ? "Some selected files are not linked. Deselect them to unlink." : false,
          confirm: (rows: readonly FileEntity[]) => ({ title: rows.length === 1 ? "Unlink this file?" : `Unlink ${rows.length} files?`, body: "Files stay on disk and in the library's file list, but are no longer attached to their items.", confirmLabel: "Unlink", tone: "warning" as const }),
          run: (rows: readonly FileEntity[]) => run(rows, "Unlink", async (row) => {
            const { data } = await unmatchFile({ variables: { mediaFileId: row.id } });
            return { ok: Boolean(data?.unmatchMediaFile.success), message: data?.unmatchMediaFile.reason };
          }), successMessage: () => "Files unlinked",
        },
        {
          id: "delete", label: (rows: readonly FileEntity[]) => rows.length === 1 ? "Delete file" : `Delete ${rows.length} files`, icon: IconTrash, group: "danger" as const, bulk: true,
          confirm: (rows: readonly FileEntity[]) => ({
            title: rows.length === 1 ? `Delete ${fileName(rows[0]!.path)} from disk?` : `Delete ${rows.length} files from disk?`,
            body: `Permanently deletes ${rows.length === 1 ? "the file" : "these files"} from disk and removes ${rows.length === 1 ? "it" : "them"} from the library. This can't be undone.`,
            confirmLabel: rows.length === 1 ? "Delete file" : `Delete ${rows.length} files`,
            tone: "danger" as const,
          }),
          run: (rows: readonly FileEntity[]) => run(rows, "Delete", async (row) => {
            const { data } = await deleteFile({ variables: { mediaFileId: row.id } });
            const ok = Boolean(data?.deleteMediaFileFromDisk.success);
            if (ok) closeFileDialog();
            return { ok, message: data?.deleteMediaFileFromDisk.reason };
          }), successMessage: (rows: readonly FileEntity[]) => rows.length === 1 ? "File deleted" : "Files deleted",
        },
      ] : []),
    ]);
  }, [analyzeFile, deleteFile, evaluate, inDialog, isAdmin, matchFile, organizeFile, unmatchFile]);

  const dialogs = <ManualMatchDialog file={matching} libraryId={matching?.libraryId ?? ""} onClose={() => setMatching(null)} onMatched={() => changed.current?.()} />;
  return { actions, dialogs };
}
