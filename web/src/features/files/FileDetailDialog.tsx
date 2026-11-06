import { ActionBar } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { Dialog } from "glass-ux/dialog";
import { IconFile } from "glass-ux/icons";

import { EntityChip } from "@/components/entities/EntityChip";
import { EntityDetailDialog } from "@/components/entities/EntityDetailDialog";
import { ErrorState, SkeletonBlock } from "@/components/ui";
import { FileSummary, fileSections, useFileStreams } from "@/features/media/FileDetails";
import { MediaFileDetailDocument } from "@/graphql/generated/graphql";
import { badgeTone, fileName, linkedItem } from "@/lib/entities/linked";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { qualityStatus } from "@/lib/status";

import { closeFileDialog, useOpenFileId } from "./fileDialog";
import { FileIdentity } from "./FileIdentity";
import { isFileLinked, useFileActions, type FileEntity } from "./useFileActions";

/** The one file dialog. Mounted once in the shell; anything that names a file opens it. */
export function FileDialogHost() {
  const id = useOpenFileId();
  const { actions, dialogs } = useFileActions(undefined, { inDialog: true });
  return (
    <>
      {id ? <FileDialog key={id} id={id} actions={actions} /> : null}
      {dialogs}
    </>
  );
}

function FileDialog({ id, actions }: { id: string; actions: ReturnType<typeof useFileActions>["actions"] }) {
  const query = useQuery(MediaFileDetailDocument, { variables: { id } });
  const streams = useFileStreams(id);
  const file = query.data?.mediaFile as FileEntity | null | undefined;
  const onOpenChange = (open: boolean) => { if (!open) closeFileDialog(); };

  if (!file) {
    return (
      <Dialog.Root open onOpenChange={onOpenChange}>
        <Dialog.Popup size="lg">
          <Dialog.Header title="File details" icon={<IconFile />} />
          <Dialog.Body>
            {query.error ? <ErrorState error={query.error} onRetry={() => void query.refetch()} />
              : query.loading ? <SkeletonBlock className="h-64 rounded-xl" />
              : <p className="text-body-sm text-muted">This file is no longer in the library.</p>}
          </Dialog.Body>
        </Dialog.Popup>
      </Dialog.Root>
    );
  }

  const item = linkedItem(file);
  const quality = qualityStatus(file.qualityStatus);
  return (
    <EntityDetailDialog
      open
      onOpenChange={onOpenChange}
      label="File details"
      identity={<FileIdentity tier="header" file={file} />}
      actions={<ActionBar set={actions} targets={[file]} label={`Actions for ${fileName(file.path)}`} maxButtons={2} />}
      badges={
        <>
          {item ? <EntityChip entity={item} /> : <Badge size="sm" marker="dot" tone={isFileLinked(file) ? "success" : "warning"}>{isFileLinked(file) ? "Linked" : "Unmatched"}</Badge>}
          <Badge size="sm" marker="dot" tone={badgeTone(quality.tone)}>{quality.label}</Badge>
        </>
      }
      summary={<FileSummary file={file} />}
      sections={fileSections(file, streams)}
    />
  );
}
