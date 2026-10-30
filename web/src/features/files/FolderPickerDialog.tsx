import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { IconArrowUp, IconFolder, IconFolderOpen, IconLock } from "glass-ux/icons";
import { ListRow } from "glass-ux/list-row";
import { useEffect, useState } from "react";

import { Button, Dialog, EmptyState, Spinner } from "@/components/ui";
import { BrowseDirectoryDocument } from "@/graphql/generated/graphql";

interface FolderPickerDialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  initialPath?: string;
  onPick: (path: string) => void;
}

/** Server-side folder browser used when choosing a library root or a destination. */
export function FolderPickerDialog({ isOpen, onOpenChange, initialPath, onPick }: FolderPickerDialogProps) {
  const [path, setPath] = useState<string | null>(initialPath || null);
  useEffect(() => {
    if (isOpen) setPath(initialPath || null);
  }, [isOpen, initialPath]);

  const { data, previousData, loading, error } = useQuery(BrowseDirectoryDocument, { variables: { input: { path, dirsOnly: true, showHidden: false } }, skip: !isOpen });
  const listing = (data ?? previousData)?.browseDirectory;

  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title="Choose a folder"
      size="md"
      footer={
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            variant="primary"
            isDisabled={!listing || loading || Boolean(error)}
            onPress={() => {
              if (listing) onPick(listing.currentPath);
              onOpenChange(false);
            }}
          >
            Use this folder
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2">
          <Button variant="secondary" size="sm" isIconOnly aria-label="Parent folder" isDisabled={!listing?.parentPath} onPress={() => listing?.parentPath && setPath(listing.parentPath)}>
            <IconArrowUp />
          </Button>
          <code className="min-w-0 flex-1 truncate rounded-lg bg-surface-secondary px-3 py-2 font-mono text-label text-foreground">{listing?.currentPath ?? path ?? "/"}</code>
        </div>
        {listing?.quickPaths.length ? (
          <div className="flex flex-wrap gap-1.5">
            {listing.quickPaths.map((quick) => (
              <Button key={quick.path} size="sm" variant="ghost" onPress={() => setPath(quick.path)}>
                <IconFolder /> {quick.name}
              </Button>
            ))}
          </div>
        ) : null}
        <div className="scrollbar-thin max-h-80 min-h-48 overflow-y-auto rounded-card border border-border">
          {loading && !listing ? (
            <div className="grid h-48 place-items-center">
              <Spinner />
            </div>
          ) : error ? (
            <EmptyState compact icon={IconLock} title="Can't read this folder" description={error.message} />
          ) : listing && listing.entries.length === 0 ? (
            <EmptyState compact icon={IconFolderOpen} title="Empty folder" />
          ) : (
            <ul>
              {listing?.entries.map((entry) => (
                <li key={entry.path}>
                  <ListRow
                    disabled={!entry.readable}
                    selected={entry.path === listing.currentPath}
                    onClick={() => setPath(entry.path)}
                    onDoubleClick={() => {
                      if (!entry.readable) return;
                      onPick(entry.path);
                      onOpenChange(false);
                    }}
                    icon={<IconFolder />}
                    title={entry.name}
                    trailing={!entry.writable ? <IconLock size="sm" className="text-muted" /> : undefined}
                    className="w-full"
                    data-focusable
                  />
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </Dialog>
  );
}
