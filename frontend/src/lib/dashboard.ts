import type { DashboardRecentMediaQuery } from "./graphql/generated/graphql";
import { parseTimestamp } from "./format";

export interface RecentMedia {
  id: string;
  title: string;
  imageUrl: string | null;
  createdAt: string;
  href: string;
  kind: "TV show" | "Movie" | "Album" | "Audiobook";
}

export function recentDashboardMedia(
  data: DashboardRecentMediaQuery | undefined,
): RecentMedia[] {
  if (!data) return [];
  const items: RecentMedia[] = [
    ...data.shows.edges.map(({ node }) => ({
      ...node,
      title: node.name,
      imageUrl: node.posterUrl,
      href: `/shows/${node.id}`,
      kind: "TV show" as const,
    })),
    ...data.movies.edges.map(({ node }) => ({
      ...node,
      imageUrl: node.collectionPosterUrl,
      href: `/movies/${node.id}`,
      kind: "Movie" as const,
    })),
    ...data.albums.edges.map(({ node }) => ({
      ...node,
      title: node.name,
      imageUrl: node.coverUrl,
      href: `/albums/${node.id}`,
      kind: "Album" as const,
    })),
    ...data.audiobooks.edges.map(({ node }) => ({
      ...node,
      imageUrl: node.coverUrl,
      href: `/audiobooks/${node.id}`,
      kind: "Audiobook" as const,
    })),
  ];
  return items
    .sort(
      (a, b) =>
        (parseTimestamp(b.createdAt)?.getTime() ?? 0) -
        (parseTimestamp(a.createdAt)?.getTime() ?? 0),
    )
    .slice(0, 6);
}
