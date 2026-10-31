import { defineEntityKinds, type EntityKindSpec } from "glass-ux/ai";
import type { IconComponent, IconProps } from "glass-ux/icon";
import { IconAlerts, IconDownload, IconFileVideo, IconLayers, IconPlug, IconUser, IconWarning } from "glass-ux/icons";
import { IconBooks, IconDeviceTv, IconDisc, IconMicrophone2, IconMovie, IconMusic, IconStack2, IconUserStar, type TablerIcon } from "@tabler/icons-react";

/** Tabler glyphs (media kinds Glass UX has no icon for) in Glass's icon contract. */
export function glyph(Icon: TablerIcon, name: string): IconComponent {
  const size = { xs: 12, sm: 14, md: 16, lg: 20, xl: 24, "2xl": 32 } as const;
  const component = ({ size: token, label, tint: _tint, ...rest }: IconProps) => (
    <Icon size={token ? size[token] : "1em"} stroke={1.75} aria-hidden={label ? undefined : true} aria-label={label} {...(rest as object)} />
  );
  return Object.assign(component, { iconName: name });
}

const kind = (icon: IconComponent, noun: string, plural: string, segment: string): EntityKindSpec => ({ icon, noun, plural, segment });

/**
 * Every kind of record the app names and links, declared once (docs/entity-tables.md). The
 * glyph, nouns and route segment are shared by identity cells, entity chips, previews and the
 * command palette; `segment` is the first path segment of the record's page.
 */
export const ENTITY_KINDS = defineEntityKinds({
  movie: kind(glyph(IconMovie, "movie"), "Movie", "Movies", "movies"),
  show: kind(glyph(IconDeviceTv, "show"), "Show", "Shows", "shows"),
  episode: kind(glyph(IconDeviceTv, "episode"), "Episode", "Episodes", "episodes"),
  album: kind(glyph(IconDisc, "album"), "Album", "Albums", "albums"),
  artist: kind(glyph(IconMicrophone2, "artist"), "Artist", "Artists", "artists"),
  track: kind(glyph(IconMusic, "track"), "Track", "Tracks", "tracks"),
  audiobook: kind(glyph(IconBooks, "audiobook"), "Audiobook", "Audiobooks", "audiobooks"),
  person: kind(glyph(IconUserStar, "person"), "Person", "People", "people"),
  collection: kind(glyph(IconStack2, "collection"), "Collection", "Collections", "collections"),
  library: kind(IconLayers, "Library", "Libraries", "libraries"),
  torrent: kind(IconDownload, "Download", "Downloads", "downloads"),
  mediaFile: kind(IconFileVideo, "File", "Files", "files"),
  notification: kind(IconAlerts, "Notification", "Notifications", "activity"),
  scanIssue: kind(IconWarning, "Scan issue", "Scan issues", "scans"),
  source: kind(IconPlug, "Source", "Sources", "settings/sources"),
  user: kind(IconUser, "User", "Users", "settings/users"),
});

export type EntityKindName = keyof typeof ENTITY_KINDS;

/** The record's page. Episodes open their show, which owns the season and episode views. */
export function entityPath(kind: EntityKindName, id: string, parentId?: string | null): string | undefined {
  switch (kind) {
    case "episode": return parentId ? `/shows/${parentId}` : undefined;
    case "torrent": return `/downloads?torrent=${encodeURIComponent(id)}`;
    case "notification": return "/activity";
    case "scanIssue": return undefined;
    case "mediaFile": return undefined;
    default: return `/${ENTITY_KINDS[kind].segment}/${id}`;
  }
}
