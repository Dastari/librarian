import { Dialog } from "glass-ux/dialog";
import { IconFile } from "glass-ux/icons";

import { EntityChip } from "@/components/entities/EntityChip";
import { ErrorState, SkeletonBlock } from "@/components/ui";
import { FileDetailsBody } from "@/features/media/FileDetails";
import { MediaFileDetailDocument } from "@/graphql/generated/graphql";
import { linkedItem } from "@/lib/entities/linked";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { closeFileDialog, useOpenFileId } from "./fileDialog";
import { FileIdentity } from "./FileIdentity";
import { useFileActions, type FileEntity } from "./useFileActions";

/** The one file dialog. Mounted once in the shell; anything that names a file opens it. */
export function FileDialogHost() {
  const id = useOpenFileId();
  const { actions, dialogs } = useFileActions(undefined, { inDialog: true });
  return (
    <>
      <Dialog.Root open={Boolean(id)} onOpenChange={(open) => { if (!open) closeFileDialog(); }}>
        <Dialog.Popup size="lg">
          <Dialog.Header title="File details" icon={<IconFile />} close />
          <Dialog.Body className="flex min-w-0 flex-col gap-6">{id ? <FileDialogContent id={id} actions={actions} /> : null}</Dialog.Body>
        </Dialog.Popup>
      </Dialog.Root>
      {dialogs}
    </>
  );
}

function FileDialogContent({ id, actions }: { id: string; actions: ReturnType<typeof useFileActions>["actions"] }) {
  const query = useQuery(MediaFileDetailDocument, { variables: { id } });
  const file = query.data?.mediaFile as FileEntity | null | undefined;
  if (query.error && !file) return <ErrorState error={query.error} onRetry={() => void query.refetch()} />;
  if (!file) {
    if (query.loading) return <SkeletonBlock className="h-64 rounded-xl" />;
    return <p className="text-body-sm text-muted">This file is no longer in the library.</p>;
  }
  const item = linkedItem(file);
  return (
    <>
      <FileIdentity file={file} actions={actions} tier="header" />
      {item ? <div className="flex min-w-0 items-center gap-2 text-label-sm text-muted">Linked to <EntityChip entity={item} /></div> : null}
      <FileDetailsBody file={file} />
    </>
  );
}
