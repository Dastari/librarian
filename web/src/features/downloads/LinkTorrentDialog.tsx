import { useMutation } from "@apollo/client/react";
import { Button } from "glass-ux/button";
import { Dialog } from "glass-ux/dialog";
import { IconCheck, IconLink } from "glass-ux/icons";
import { ListRow } from "glass-ux/list-row";
import { toast } from "glass-ux/toast";
import { useEffect, useState } from "react";

import { ErrorState, SkeletonBlock } from "@/components/ui";
import { useDialogReturnFocus } from "@/components/ui/Dialog";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { EntityLibraryListDocument, EntityTorrentUpdateDocument, RematchSourceDocument } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { libraryType } from "@/lib/library-types";

import { TorrentIdentity } from "./TorrentIdentity";
import type { DownloadRow } from "./useDownloads";

interface LinkTorrentDialogProps {
  row: DownloadRow | null;
  onClose: () => void;
  onLinked?: () => void;
}

/** Points a download at a library and re-runs matching so its files import there. */
export function LinkTorrentDialog({ row, onClose, onLinked }: LinkTorrentDialogProps) {
  const open = Boolean(row);
  const finalFocus = useDialogReturnFocus(open);
  const libraries = useCompleteConnection(EntityLibraryListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.libraries, { skip: !row });
  const [libraryId, setLibraryId] = useState<string | null>(null);
  const [updateTorrent, { loading: saving }] = useMutation(EntityTorrentUpdateDocument);
  const [rematch, { loading: matching }] = useMutation(RematchSourceDocument);
  const current = row?.record.libraryId ?? null;
  const finished = row ? (row.live?.progress ?? row.record.progress) >= 1 : false;
  const busy = saving || matching;

  useEffect(() => {
    setLibraryId(current);
  }, [row?.record.id, current]);

  const save = async (next: string | null) => {
    if (!row) return;
    try {
      const updated = assertSuccess((await updateTorrent({ variables: { id: row.record.id, input: { libraryId: next } } })).data?.updateTorrent, "Could not link");
      // A generated field policy may drop a rejected nullable field while the
      // mutation still reports success. Confirm persistence before any follow-up.
      if (!updated.torrent || updated.torrent.id !== row.record.id || updated.torrent.libraryId !== next) {
        throw new Error("The download's library link was not saved. Refresh and try again.");
      }
      if (next && finished) {
        const result = assertSuccess((await rematch({ variables: { sourceId: row.record.infoHash, sourceType: "torrent", libraryId: next } })).data?.rematchSource, "Matching failed");
        toast.success(`Linked and matched ${result.matchCount} ${result.matchCount === 1 ? "file" : "files"}`);
      } else toast.success(next ? "Linked" : "Link removed");
      onLinked?.();
      onClose();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <Dialog.Root open={open} onOpenChange={(next) => { if (!next) onClose(); }}>
      <Dialog.Popup size="md" finalFocus={finalFocus}>
        <Dialog.Header title="Link to library" icon={<IconLink />} />
        <Dialog.Body className="flex min-w-0 flex-col gap-4">
          {row ? <TorrentIdentity row={row} className="min-w-0" /> : null}
          <p className="text-body-sm text-muted">{finished ? "Choose where this download belongs. Each of its files is then matched against that library." : "Choose where this download belongs. Its files are matched against that library when it finishes."}</p>
          {libraries.error ? <ErrorState error={libraries.error} onRetry={() => void libraries.refetch()} compact />
            : libraries.loading ? <SkeletonBlock className="h-32 rounded-xl" />
            : libraries.rows.length === 0 ? <p className="text-body-sm text-muted">Create a library in Settings first.</p>
            : (
              <div role="radiogroup" aria-label="Library" className="flex min-w-0 flex-col gap-1">
                {libraries.rows.map((library) => {
                  const meta = libraryType(library.libraryType);
                  const chosen = library.id === libraryId;
                  return <ListRow key={library.id} role="radio" aria-checked={chosen} selected={chosen} data-focusable icon={<meta.icon />} title={library.name} meta={library.path} trailing={chosen ? <IconCheck size="sm" /> : undefined} onClick={() => setLibraryId(library.id)} />;
                })}
              </div>
            )}
        </Dialog.Body>
        <Dialog.Footer>
          {current ? <Button variant="ghost" className="me-auto" disabledReason={busy ? "Saving" : undefined} onClick={() => void save(null)}>Remove link</Button> : null}
          <Dialog.Close render={<Button variant="ghost">Cancel</Button>} />
          <Button variant="primary" status={busy ? "pending" : undefined} disabledReason={!libraryId ? "Choose a library" : undefined} onClick={() => void save(libraryId)}>{finished ? "Link and match" : "Link"}</Button>
        </Dialog.Footer>
      </Dialog.Popup>
    </Dialog.Root>
  );
}
