import { useMutation } from "@apollo/client/react";
import { Select } from "glass-ux/select";
import { toast } from "glass-ux/toast";
import { useEffect, useState } from "react";

import { Button, Dialog, ErrorState } from "@/components/ui";
import { EntityTorrentUpdateDocument, EntityLibraryListDocument, RematchSourceDocument } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";

import type { DownloadRow } from "./useDownloads";

interface LinkTorrentDialogProps {
  row: DownloadRow | null;
  onClose: () => void;
  onLinked?: () => void;
}

/** Points a download at a library and re-runs matching so its files import there. */
export function LinkTorrentDialog({ row, onClose, onLinked }: LinkTorrentDialogProps) {
  const libraries = useCompleteConnection(EntityLibraryListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.libraries, { skip: !row });
  const [libraryId, setLibraryId] = useState<string | null>(null);
  const [updateTorrent, { loading: saving }] = useMutation(EntityTorrentUpdateDocument);
  const [rematch, { loading: matching }] = useMutation(RematchSourceDocument);

  useEffect(() => {
    setLibraryId(row?.record?.libraryId ?? null);
  }, [row?.record?.id, row?.record?.libraryId]);

  const save = async () => {
    if (!row?.record) {
      toast.warning("This download has no record yet. Try again in a moment.");
      return;
    }
    try {
      const updated = assertSuccess((await updateTorrent({ variables: { id: row.record.id, input: { libraryId } } })).data?.updateTorrent, "Could not link");
      // A generated field policy may drop a rejected nullable field while the
      // mutation still reports success. Confirm persistence before any follow-up.
      if (!updated.torrent || updated.torrent.id !== row.record.id || updated.torrent.libraryId !== libraryId) {
        throw new Error("The download's library link was not saved. Refresh and try again.");
      }
      if (libraryId && (row.live?.progress ?? row.record.progress) >= 1) {
        const result = assertSuccess((await rematch({ variables: { sourceId: row.record.id, sourceType: "torrent", libraryId } })).data?.rematchSource, "Matching failed");
        toast.success(`Linked and matched ${result.matchCount} files`);
      } else toast.success(libraryId ? "Linked" : "Unlinked");
      onLinked?.();
      onClose();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <Dialog
      isOpen={Boolean(row)}
      onOpenChange={(open) => !open && onClose()}
      title="Link to library"
      description={row?.record.name}
      size="sm"
      footer={
        <>
          <Button variant="ghost" onPress={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onPress={() => void save()} isPending={saving || matching || libraries.loading} isDisabled={Boolean(libraries.error)}>
            Link
          </Button>
        </>
      }
    >
      {libraries.error ? <ErrorState error={libraries.error} onRetry={() => void libraries.refetch()} compact /> : null}
      <Select aria-label="Library" value={libraryId} onValueChange={setLibraryId} placeholder="Choose a library" width="full" disabled={libraries.loading} options={libraries.rows.map((library) => ({ value: library.id, label: library.name }))} />
    </Dialog>
  );
}
