import { useMutation, useQuery } from "@apollo/client/react";
import { ListBox, ListBoxItem, Select, toast } from "@heroui/react";
import { useEffect, useState } from "react";

import { Button, Dialog } from "@/components/ui";
import { EntityTorrentUpdateDocument, NavLibrariesDocument, RematchSourceDocument } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import type { TorrentRow } from "./useLiveTorrents";

interface LinkTorrentDialogProps {
  row: TorrentRow | null;
  onClose: () => void;
  onLinked?: () => void;
}

/** Points a download at a library and re-runs matching so its files import there. */
export function LinkTorrentDialog({ row, onClose, onLinked }: LinkTorrentDialogProps) {
  const libraries = useQuery(NavLibrariesDocument, { skip: !row });
  const [libraryId, setLibraryId] = useState<string | null>(null);
  const [updateTorrent, { loading: saving }] = useMutation(EntityTorrentUpdateDocument);
  const [rematch, { loading: matching }] = useMutation(RematchSourceDocument);

  useEffect(() => {
    setLibraryId(row?.record?.libraryId ?? null);
  }, [row]);

  const save = async () => {
    if (!row?.record) {
      toast.warning("This download has no record yet. Try again in a moment.");
      return;
    }
    try {
      assertSuccess((await updateTorrent({ variables: { id: row.record.id, input: { libraryId } } })).data?.updateTorrent, "Could not link");
      if (libraryId && row.live.progress >= 1) {
        const result = assertSuccess((await rematch({ variables: { sourceId: row.record.id, sourceType: "torrent", libraryId } })).data?.rematchSource, "Matching failed");
        toast.success(`Linked and matched ${result.matchCount} files`);
      } else toast.success("Linked");
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
      description={row?.live.name}
      size="sm"
      footer={
        <>
          <Button variant="ghost" onPress={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onPress={() => void save()} isPending={saving || matching}>
            Link
          </Button>
        </>
      }
    >
      <Select aria-label="Library" selectedKey={libraryId} onSelectionChange={(key) => setLibraryId(key === null ? null : String(key))} placeholder="Choose a library" fullWidth>
        <Select.Trigger>
          <Select.Value />
          <Select.Indicator />
        </Select.Trigger>
        <Select.Popover>
          <ListBox>
            {(libraries.data?.libraries.edges ?? []).map(({ node }) => (
              <ListBoxItem key={node.id} id={node.id} textValue={node.name}>
                {node.name}
              </ListBoxItem>
            ))}
          </ListBox>
        </Select.Popover>
      </Select>
    </Dialog>
  );
}
