import { useApolloClient } from "@apollo/client/react";
import { FileDialog, type FileDialogPlace } from "glass-ux/file-dialog";
import type { DirectoryPage, FileEntry, FileProvider, FileStat, FileUri } from "glass-ux/file-provider";
import { useMemo } from "react";

import { BrowseDirectoryDocument } from "@/graphql/generated/graphql";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

const ROOT: FileUri = "server:///";

/**
 * A folder on a network share (`smb://host/share/folder`) is one path segment `smb:` followed by
 * its parts, so it nests under the server's own folders the way a Glass location expects.
 */
const SMB_SEGMENT = "smb:";

export const folderUri = (path: string): FileUri => {
  if (path.startsWith("smb://")) return `server:///${[SMB_SEGMENT, ...path.slice("smb://".length).split("/")].map(encodeURIComponent).join("/")}`;
  return `server://${path.startsWith("/") ? "" : "/"}${path.split("/").map(encodeURIComponent).join("/")}`;
};

export const folderPath = (uri: FileUri): string => {
  const path = decodeURIComponent(uri.replace(/^server:\/\//, "")) || "/";
  return path.startsWith(`/${SMB_SEGMENT}/`) ? `smb://${path.slice(SMB_SEGMENT.length + 2)}` : path;
};

/** `smb://host/share` of a path on a share, which a picker inside a share cannot climb out of. */
export const shareRootOf = (path: string | undefined): string | null => /^(smb:\/\/[^/]+\/[^/]+)/i.exec(path ?? "")?.[1] ?? null;

interface FolderPickerDialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  initialPath?: string;
  onPick: (path: string) => void;
}

/** The server's folders as a Glass file location: it lists them, and the dialog does the rest. */
function useServerFolders(rootPath: string): FileProvider {
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
      root: rootPath === "/" ? ROOT : folderUri(rootPath),
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
  }, [client, rootPath]);
}

/** Choose a folder on the server, for a library's root or a destination. */
export function FolderPickerDialog({ isOpen, onOpenChange, initialPath, onPick }: FolderPickerDialogProps) {
  // Inside a share the picker stays inside it: there is nothing above a share to browse.
  const shareRoot = shareRootOf(initialPath);
  const provider = useServerFolders(shareRoot ?? "/");
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
