import type { LibraryType } from "@/lib/library-types";

/** The search scope that adds new items of a library type from a metadata provider. */
export const ADD_SCOPES = {
  movies: { scope: "movie", label: "Add movies" },
  tv: { scope: "show", label: "Add TV shows" },
  music: { scope: "album", label: "Add music" },
  audiobooks: { scope: "audiobook", label: "Add audiobooks" },
} as const satisfies Partial<Record<LibraryType, { scope: string; label: string }>>;

export type AddableType = keyof typeof ADD_SCOPES;

export function addScopeFor(type: LibraryType): (typeof ADD_SCOPES)[AddableType] | null {
  return type in ADD_SCOPES ? ADD_SCOPES[type as AddableType] : null;
}

/** Search, opened on the tab that adds items of this type. */
export function addHref(type: LibraryType): string | null {
  const add = addScopeFor(type);
  return add ? `/search?in=${add.scope}` : null;
}
