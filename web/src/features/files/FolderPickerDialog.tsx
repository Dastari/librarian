import { useApolloClient } from "@apollo/client/react";
import { FileDialog, type FileDialogPlace } from "glass-ux/file-dialog";
import type { DirectoryPage, FileEntry, FileProvider, FileStat, FileUri } from "glass-ux/file-provider";
import { useMemo } from "react";

import { BrowseDirectoryDocument } from "@/graphql/generated/graphql";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

const ROOT: FileUri = "server:///";

export const folderUri = (path: string): FileUri =>
  `server://${path.startsWith("/") ? "" : "/"}${path.split("/").map(encodeURIComponent).join("/")}`;

export const folderPath = (uri: FileUri): string =>
  decodeURIComponent(uri.replace(/^server:\/\//, "")) || "/";

interface FolderPickerDialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  initialPath?: string;
  onPick: (path: string) => void;
}

/** The server's folders as a Glass file location: it lists them, and the dialog does the rest. */
function useServerFolders(): FileProvider {
  const client = useApolloClient();
  return useMemo(() => {
    const browse = async (uri: FileUri) => {
      const { data, error } = await client.query({
        query: BrowseDirectoryDocument,
        variables: { input: { path: folderPath(uri), dirsOnly: false, showHidden: false } },
        fetchPolicy: "network-only",
      });
      if (!data) throw Object.assign(error ?? new Error("Can't read this folder"), { code: "unavailable" });
      return data.browseDirectory;
    };
    return {
      scheme: "server",
      label: "Server",
      root: ROOT,
      stat: async (uri): Promise<FileStat> => {
        const listing = await browse(uri);
        const path = listing.currentPath;
        return { uri: folderUri(path), name: path.split("/").filter(Boolean).at(-1) ?? "/", kind: "directory", size: null, version: path };
      },
      read: () => Promise.reject(Object.assign(new Error("Folders only"), { code: "unsupported" })),
      readDirectory: async (uri): Promise<DirectoryPage> => {
        const listing = await browse(uri);
        return {
          cursor: null,
          entries: listing.entries.map((entry): FileEntry => ({
            uri: folderUri(entry.path),
            name: entry.name,
            kind: entry.isDir ? "directory" : "file",
            size: entry.isDir ? null : entry.size ?? null,
            mime: entry.mimeType ?? undefined,
            denied: entry.readable ? undefined : { read: "Librarian can't read this folder" },
          })),
        };
      },
    };
  }, [client]);
}

/** Choose a folder on the server, for a library's root or a destination. */
export function FolderPickerDialog({ isOpen, onOpenChange, initialPath, onPick }: FolderPickerDialogProps) {
  const provider = useServerFolders();
  const { data } = useQuery(BrowseDirectoryDocument, {
    skip: !isOpen,
    variables: { input: { path: initialPath || null, dirsOnly: true, showHidden: false } },
  });
  const places = useMemo<FileDialogPlace[]>(
    () => (data?.browseDirectory.quickPaths ?? []).map((quick) => ({ label: quick.name, uri: folderUri(quick.path) })),
    [data],
  );
  return (
    <FileDialog
      mode="folder"
      open={isOpen}
      onOpenChange={onOpenChange}
      providers={[provider]}
      places={places}
      defaultUri={folderUri(initialPath || data?.browseDirectory.currentPath || "/")}
      onChoose={({ folder }) => {
        onPick(folderPath(folder.uri));
        onOpenChange(false);
      }}
    />
  );
}
