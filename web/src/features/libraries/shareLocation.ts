/**
 * A library's folder on a network share, as the parts a person types and the one address the
 * server keeps (`smb://host[:port]/share/folder`, docs/design.md Q85). The server is the judge
 * of what is valid; these only split and join.
 */

export interface ShareLocation {
  /** Host name or IP address, with `:port` when it is not 445. */
  host: string;
  share: string;
  /** Folder inside the share, `/`-separated, without leading or trailing slash. */
  folder: string;
}

export const isShareAddress = (value: string): boolean => /^smb:\/\//i.test(value.trim());

/** The parts of an `smb://` address, or `null` for a path on the server itself. */
export function parseShareAddress(value: string): ShareLocation | null {
  const match = /^smb:\/\/([^/]*)(?:\/([^/]*))?(?:\/(.*))?$/i.exec(value.trim());
  if (!match) return null;
  return { host: match[1] ?? "", share: match[2] ?? "", folder: (match[3] ?? "").split("/").filter(Boolean).join("/") };
}

/** The address kept for a library, or an empty string until the host and share are both given. */
export function composeShareAddress({ host, share, folder }: ShareLocation): string {
  const cleanHost = host.trim();
  const cleanShare = share.trim().replace(/^\/+|\/+$/g, "");
  if (!cleanHost || !cleanShare) return "";
  const cleanFolder = folder.split("/").map((part) => part.trim()).filter(Boolean).join("/");
  return `smb://${cleanHost}/${cleanShare}${cleanFolder ? `/${cleanFolder}` : ""}`;
}

/** `host` and optional port from what was typed in the server box (`10.0.1.190` or `nas:1445`). */
export function splitHostPort(value: string): { host: string; port: number | null } {
  const trimmed = value.trim();
  const match = /^(.*?):(\d{1,5})$/.exec(trimmed);
  if (match && !trimmed.startsWith("[")) return { host: match[1]!, port: Number(match[2]) };
  return { host: trimmed, port: null };
}

/** The share's own address, the part a saved sign-in belongs to. */
export function shareRootAddress({ host, share }: Pick<ShareLocation, "host" | "share">): string {
  return composeShareAddress({ host, share, folder: "" });
}
