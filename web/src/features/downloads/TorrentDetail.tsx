import { IconX } from "@tabler/icons-react";

import { KeyValueList, Panel, StatusChip } from "@/components/ui";
import { formatBytes, formatDateTime, formatSpeed } from "@/lib/format";
import { torrentState } from "@/lib/status";

import type { TorrentRow } from "./useLiveTorrents";

/** Side panel with the files inside a torrent and its import outcome. */
export function TorrentDetail({ row, onClose }: { row: TorrentRow; onClose: () => void }) {
  const state = torrentState(row.live.state);
  return (
    <Panel
      title={row.live.name}
      actions={
        <>
          <StatusChip status={state} />
          <button type="button" data-focusable aria-label="Close" onClick={onClose} className="nav-focus grid size-8 place-items-center rounded-full text-muted hover:bg-surface-hover hover:text-foreground">
            <IconX size={16} />
          </button>
        </>
      }
      className="self-start xl:sticky xl:top-4"
    >
      <KeyValueList
        items={[
          { label: "Progress", value: `${Math.round(row.live.progress * 100)}% · ${formatBytes(row.live.downloaded)} of ${formatBytes(row.live.size)}` },
          { label: "Speed", value: `↓ ${formatSpeed(row.live.downloadSpeed)} · ↑ ${formatSpeed(row.live.uploadSpeed)}` },
          { label: "Peers", value: row.live.peers },
          { label: "Uploaded", value: formatBytes(row.live.uploaded) },
          { label: "Save path", value: row.live.savePath, mono: true },
          { label: "Info hash", value: row.live.infoHash, mono: true },
          { label: "Added", value: row.record ? formatDateTime(row.record.addedAt) : undefined },
          { label: "Completed", value: row.record?.completedAt ? formatDateTime(row.record.completedAt) : undefined },
          { label: "Season", value: row.record?.season !== null && row.record?.season !== undefined ? (row.record.season === 0 ? "Specials pack" : `Season ${row.record.season} pack`) : undefined },
          { label: "Grabbed for", value: targetLabel(row.record) },
          { label: "Import", value: row.record?.postProcessStatus },
          { label: "Import error", value: row.record?.postProcessError ? <span className="text-danger">{row.record.postProcessError}</span> : undefined },
        ]}
      />
      <p className="text-overline mb-2 mt-5 text-muted">Files</p>
      <ul className="scrollbar-thin max-h-80 overflow-y-auto text-body-sm">
        {row.live.files.map((file) => (
          <li key={file.index} className="flex items-center gap-3 border-b border-separator py-1.5 last:border-b-0">
            <span className="min-w-0 flex-1 truncate font-mono text-label text-foreground">{file.path.split(/[\\/]/).pop()}</span>
            <span className="text-numeric text-label-sm text-muted">{formatBytes(file.size)}</span>
            <span className="h-1 w-16 overflow-hidden rounded-full bg-surface-tertiary">
              <span className="block h-full bg-info" style={{ width: `${Math.min(100, file.progress * 100)}%` }} />
            </span>
          </li>
        ))}
      </ul>
    </Panel>
  );
}

/** What the grab was recorded against, so a stuck import can be traced back to its item. */
function targetLabel(record: TorrentRow["record"]): string | undefined {
  if (!record) return undefined;
  if (record.episodeId) return "Episode";
  if (record.showId) return "Show";
  if (record.movieId) return "Movie";
  if (record.trackId) return "Track";
  if (record.albumId) return "Album";
  if (record.chapterId) return "Chapter";
  if (record.audiobookId) return "Audiobook";
  return undefined;
}
