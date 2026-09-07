import {
  IconBooks,
  IconDeviceTv,
  IconFolder,
  IconMovie,
  IconMusic,
  type Icon as TablerIcon,
} from "@tabler/icons-react";

/** Library types as stored by the backend (lower-case). */
export type LibraryType = "movies" | "tv" | "music" | "audiobooks" | "other";

export interface LibraryTypeMeta {
  type: LibraryType;
  label: string;
  singular: string;
  icon: TablerIcon;
  /** Tailwind text colour class using the media tint tokens. */
  tint: string;
  /** CSS variable used by placeholder gradients. */
  tintVar: string;
  /** Poster aspect used by cards for the primary item type. */
  aspect: "poster" | "square";
}

export const LIBRARY_TYPES: Record<LibraryType, LibraryTypeMeta> = {
  movies: {
    type: "movies",
    label: "Movies",
    singular: "Movie",
    icon: IconMovie,
    tint: "text-media-movies",
    tintVar: "var(--media-movies)",
    aspect: "poster",
  },
  tv: {
    type: "tv",
    label: "TV Shows",
    singular: "Show",
    icon: IconDeviceTv,
    tint: "text-media-tv",
    tintVar: "var(--media-tv)",
    aspect: "poster",
  },
  music: {
    type: "music",
    label: "Music",
    singular: "Album",
    icon: IconMusic,
    tint: "text-media-music",
    tintVar: "var(--media-music)",
    aspect: "square",
  },
  audiobooks: {
    type: "audiobooks",
    label: "Audiobooks",
    singular: "Audiobook",
    icon: IconBooks,
    tint: "text-media-audiobooks",
    tintVar: "var(--media-audiobooks)",
    aspect: "poster",
  },
  other: {
    type: "other",
    label: "Files",
    singular: "File",
    icon: IconFolder,
    tint: "text-media-other",
    tintVar: "var(--media-other)",
    aspect: "square",
  },
};

export function libraryType(value: string | null | undefined): LibraryTypeMeta {
  const key = (value ?? "other").toLowerCase();
  if (key === "shows" || key === "tv_shows") return LIBRARY_TYPES.tv;
  return LIBRARY_TYPES[key as LibraryType] ?? LIBRARY_TYPES.other;
}

export const LIBRARY_TYPE_OPTIONS = (Object.values(LIBRARY_TYPES) as LibraryTypeMeta[]).filter((meta) => meta.type !== "other");
