import { formatRelative } from "@/lib/format";

/** What a library says about its folder watch (`docs/design.md` Q117). */
export interface LibraryWatchFields {
  path: string;
  watchForChanges: boolean;
  watchStatus?: string | null;
  watchDetail?: string | null;
  watchStatusSince?: string | null;
}

export interface LibraryWatchState {
  id: "off" | "starting" | "watching" | "unavailable" | "failed";
  label: string;
  tone: "neutral" | "info" | "success" | "warning" | "danger";
  /** One line of why, when there is something to say. */
  detail?: string;
}

/** The watch as a person reads it: on and listening, waiting for the share, or unable to watch. */
export function libraryWatch(library: LibraryWatchFields, now = Date.now()): LibraryWatchState {
  if (!library.watchForChanges) return { id: "off", label: "Not watched", tone: "neutral" };
  const share = library.path.startsWith("smb://");
  const since = library.watchStatusSince ? `since ${formatRelative(library.watchStatusSince, now)}` : "";
  switch (library.watchStatus) {
    case "watching":
      return { id: "watching", label: "Watching for changes", tone: "success" };
    case "unavailable":
      return {
        id: "unavailable",
        label: share ? "Share not reachable, retrying" : "Folder not reachable, retrying",
        tone: "warning",
        detail: [since && `Unreachable ${since}.`, "Changes made meanwhile are caught up when it's back."].filter(Boolean).join(" "),
      };
    case "failed":
      return { id: "failed", label: "Can't watch the folder", tone: "danger", detail: library.watchDetail ?? undefined };
    default:
      return { id: "starting", label: "Starting to watch", tone: "info" };
  }
}
