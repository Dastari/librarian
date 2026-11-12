import { StorageLocationField } from "@/features/servers/StorageLocationField";

/**
 * Where a library's files are: a folder on this server, or a folder on a share of a saved file
 * server (Settings › Network shares). Goes in a form's `path` field.
 */
export function LibraryLocationField({ required, localPlaceholder }: { required?: boolean; localPlaceholder?: string }) {
  return <StorageLocationField required={required} localPlaceholder={localPlaceholder} />;
}
