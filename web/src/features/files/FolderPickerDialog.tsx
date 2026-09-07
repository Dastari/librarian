import { useQuery } from "@apollo/client/react";

import { IconArrowUp, IconFolder, IconFolderOpen, IconLock } from "@tabler/icons-react";
import { useEffect, useState } from "react";

import { Button, Dialog, EmptyState, Spinner } from "@/components/ui";
import { BrowseDirectoryDocument } from "@/graphql/generated/graphql";
import { cn } from "@/lib/utils";

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

  const { data, loading, error } = useQuery(BrowseDirectoryDocument, { variables: { input: { path, dirsOnly: true, showHidden: false } }, skip: !isOpen });
  const listing = data?.browseDirectory;

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
            isDisabled={!listing}
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
            <IconArrowUp size={16} />
          </Button>
          <code className="min-w-0 flex-1 truncate rounded-lg bg-surface-secondary px-3 py-2 font-mono text-label text-foreground">{listing?.currentPath ?? path ?? "/"}</code>
        </div>
        {listing?.quickPaths.length ? (
          <div className="flex flex-wrap gap-1.5">
            {listing.quickPaths.map((quick) => (
              <Button key={quick.path} size="sm" variant="ghost" onPress={() => setPath(quick.path)}>
                <IconFolder size={14} className="text-media-audiobooks" /> {quick.name}
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
                  <button
                    type="button"
                    data-focusable
                    disabled={!entry.readable}
                    onClick={() => setPath(entry.path)}
                    onDoubleClick={() => {
                      onPick(entry.path);
                      onOpenChange(false);
                    }}
                    className={cn("nav-focus flex w-full items-center gap-3 px-3 py-2 text-left text-body-sm hover:bg-surface-hover disabled:opacity-50", entry.path === listing.currentPath && "bg-brand-soft")}
                  >
                    <IconFolder size={18} className="shrink-0 text-media-audiobooks" />
                    <span className="min-w-0 flex-1 truncate">{entry.name}</span>
                    {!entry.writable ? <IconLock size={14} className="text-muted" /> : null}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </Dialog>
  );
}
