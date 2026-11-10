import type { ArtworkEntityKind, ArtworkKind } from "@/graphql/generated/graphql";
import type { ArtworkAspect } from "@/components/ui/Artwork";
import type { ArtworkEntity, ArtworkKind as ArtworkPathKind } from "@/lib/api/urls";

/** A title whose pictures can be changed. */
export interface ArtworkTarget {
  entityType: ArtworkEntityKind;
  entityId: string;
  /** Shown in the picker's title: "Artwork for Heat". */
  title: string;
}

/** How a slot reads and looks in the picker. */
export interface SlotInfo {
  kind: ArtworkKind;
  label: string;
  /** "poster" for the tab, "a poster" in a sentence. */
  noun: string;
  aspect: ArtworkAspect;
  /** Logos have transparency and must not be cropped. */
  contain?: boolean;
  /** The artwork cache's path names, for previews of what's shown now. */
  path: [ArtworkEntity, ArtworkPathKind];
}

const MOVIE: SlotInfo[] = [
  { kind: "POSTER", label: "Poster", noun: "poster", aspect: "poster", path: ["movie", "poster"] },
  { kind: "BACKDROP", label: "Backdrop", noun: "backdrop", aspect: "backdrop", path: ["movie", "backdrop"] },
  { kind: "LOGO", label: "Logo", noun: "logo", aspect: "backdrop", contain: true, path: ["movie", "logo"] },
];

/** The pictures each kind of title has, in the order the picker shows them. */
export const SLOTS: Record<ArtworkEntityKind, SlotInfo[]> = {
  MOVIE,
  SHOW: [
    { kind: "POSTER", label: "Poster", noun: "poster", aspect: "poster", path: ["show", "poster"] },
    { kind: "BACKDROP", label: "Backdrop", noun: "backdrop", aspect: "backdrop", path: ["show", "backdrop"] },
  ],
  SEASON: [{ kind: "POSTER", label: "Poster", noun: "poster", aspect: "poster", path: ["season", "poster"] }],
  EPISODE: [{ kind: "THUMBNAIL", label: "Still", noun: "still", aspect: "backdrop", path: ["episode", "thumbnail"] }],
  ALBUM: [{ kind: "COVER", label: "Cover", noun: "cover", aspect: "square", path: ["album", "cover"] }],
  ARTIST: [{ kind: "POSTER", label: "Picture", noun: "picture", aspect: "square", path: ["artist", "poster"] }],
  AUDIOBOOK: [{ kind: "COVER", label: "Cover", noun: "cover", aspect: "square", path: ["audiobook", "cover"] }],
  PLAYLIST: [{ kind: "COVER", label: "Cover", noun: "cover", aspect: "square", path: ["playlist", "cover"] }],
  USER: [{ kind: "AVATAR", label: "Photo", noun: "photo", aspect: "square", path: ["user", "avatar"] }],
};

export const slotInfo = (entityType: ArtworkEntityKind, kind: ArtworkKind): SlotInfo =>
  SLOTS[entityType].find((slot) => slot.kind === kind) ?? SLOTS[entityType][0]!;

/** The artwork cache's entity name, for revisions and previews. */
export const artworkEntityPath = (entityType: ArtworkEntityKind): ArtworkEntity => SLOTS[entityType][0]!.path[0];

/** Field names of artwork locks; other locks belong to the metadata form. */
export const ARTWORK_LOCKS = ["posterUrl", "backdropUrl", "stillUrl", "coverUrl", "imageUrl"] as const;
export const isArtworkLock = (field: string) => (ARTWORK_LOCKS as readonly string[]).includes(field);

/**
 * A metadata form's locks with the entity's artwork locks as they are now: the form only edits
 * text fields, and a choice of picture made while it was open must not be unlocked by saving it.
 */
export function withArtworkLocks(formLocks: readonly string[], entityLocks: readonly string[] | null | undefined): string[] {
  return [...formLocks.filter((field) => !isArtworkLock(field)), ...(entityLocks ?? []).filter(isArtworkLock)];
}

/** What the picker accepts for an upload; the server checks the same. */
export const UPLOAD_ACCEPT = "image/jpeg,image/png,image/webp";
export const UPLOAD_MAX_BYTES = 20 * 1024 * 1024;
