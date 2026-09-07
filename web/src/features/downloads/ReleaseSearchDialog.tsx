import { useLazyQuery, useMutation, useQuery } from "@apollo/client/react";
import { SearchField, Select, ListBox, ListBoxItem, Tooltip, toast } from "@heroui/react";
import { IconDownload, IconSearch, IconX } from "@tabler/icons-react";
import { useEffect, useMemo, useState } from "react";

import { Button, DataTable, type DataTableColumn, Dialog, EmptyState, ErrorState, GlassSegmented, Spinner, StatusChip } from "@/components/ui";
import { AddTorrentDocument, EntitySourceListDocument, SearchSourcesDocument, type SearchSourcesQuery } from "@/graphql/generated/graphql";
import { formatBytes, formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { languageName } from "@/lib/languages";
import type { StatusMeta } from "@/lib/status";
import { cn } from "@/lib/utils";

type Release = SearchSourcesQuery["searchSources"]["sources"][number]["releases"][number];

type Resolution = "any" | "2160p" | "1080p" | "720p" | "sd";
type Codec = "any" | "hevc" | "h264" | "av1";
type Packs = "any" | "packs" | "episodes";

/** The backend's verdict for the resolved quality profile. */
const MATCH: Record<string, StatusMeta> = {
  optimal: { label: "Optimal", tone: "success", dot: "bg-success" },
  suboptimal: { label: "Suboptimal", tone: "warning", dot: "bg-warning" },
  rejected: { label: "Rejected", tone: "danger", dot: "bg-danger" },
};
const MATCH_RANK: Record<string, number> = { optimal: 0, suboptimal: 1, rejected: 2 };

/** Which of the segmented resolution buckets a parsed resolution belongs to. */
function resolutionBucket(resolution: string | null | undefined): Resolution {
  const value = (resolution ?? "").toLowerCase();
  if (value === "2160p" || value === "4k") return "2160p";
  if (value === "1080p" || value === "1080i") return "1080p";
  if (value === "720p") return "720p";
  return "sd";
}

/** Target the grab is recorded against, so a manual download counts for that item. */
export interface ReleaseTarget {
  showId?: string;
  episodeId?: string;
  movieId?: string;
  albumId?: string;
  trackId?: string;
  audiobookId?: string;
  chapterId?: string;
}

interface ReleaseSearchDialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  query: string;
  year?: number | null;
  imdbId?: string | null;
  season?: number | null;
  episode?: number | null;
  libraryId?: string | null;
  /** Extra hints the backend uses to build a better source query. */
  artist?: string | null;
  album?: string | null;
  author?: string | null;
  target?: ReleaseTarget;
}

/** Search configured sources for releases and send one to the download client. */
export function ReleaseSearchDialog({ isOpen, onOpenChange, query: initialQuery, year, imdbId, season, episode, libraryId, artist, album, author, target }: ReleaseSearchDialogProps) {
  const [query, setQuery] = useState(initialQuery);
  const [search, result] = useLazyQuery(SearchSourcesDocument);
  const [addTorrent] = useMutation(AddTorrentDocument);
  const [busy, setBusy] = useState<string | null>(null);
  const [resolution, setResolution] = useState<Resolution>("any");
  const [codec, setCodec] = useState<Codec>("any");
  const [packs, setPacks] = useState<Packs>("any");
  const [language, setLanguage] = useState<string>("any");
  const [hdrOnly, setHdrOnly] = useState(false);
  const [freeleechOnly, setFreeleechOnly] = useState(false);
  const [seededOnly, setSeededOnly] = useState(true);
  const sources = useQuery(EntitySourceListDocument, { variables: { where: { enabled: { eq: true } }, page: { limit: 50, offset: 0 } }, skip: !isOpen });

  const variables = useMemo(
    () => ({
      input: {
        query: year ? `${initialQuery} ${year}` : initialQuery,
        imdbId: imdbId ?? null,
        season: season ?? null,
        episode: episode !== null && episode !== undefined ? String(episode) : null,
        year: year ?? null,
        artist: artist ?? null,
        album: album ?? null,
        author: author ?? null,
        showId: target?.showId ?? null,
        movieId: target?.movieId ?? null,
        albumId: target?.albumId ?? null,
        audiobookId: target?.audiobookId ?? null,
        limit: 100,
      },
    }),
    [initialQuery, imdbId, season, episode, year, artist, album, author, target?.showId, target?.movieId, target?.albumId, target?.audiobookId],
  );

  useEffect(() => {
    if (!isOpen) return;
    setQuery(initialQuery);
    void search({ variables });
  }, [isOpen, initialQuery, variables, search]);

  const allReleases = useMemo<Release[]>(() => {
    const all = result.data?.searchSources.sources.flatMap((source) => source.releases) ?? [];
    return [...all].sort((a, b) => (MATCH_RANK[a.profileMatch ?? ""] ?? 1) - (MATCH_RANK[b.profileMatch ?? ""] ?? 1) || (b.seeders ?? 0) - (a.seeders ?? 0));
  }, [result.data]);

  const languages = useMemo(() => [...new Set(allReleases.flatMap((release) => release.parsed.languages))].sort(), [allReleases]);

  const releases = useMemo(
    () =>
      allReleases.filter((release) => {
        const parsed = release.parsed;
        if (resolution !== "any" && resolutionBucket(parsed.resolution) !== resolution) return false;
        if (codec !== "any" && parsed.codec?.toLowerCase() !== codec) return false;
        if (packs === "packs" && !parsed.isSeasonPack) return false;
        if (packs === "episodes" && parsed.isSeasonPack) return false;
        if (language !== "any" && !parsed.languages.includes(language)) return false;
        if (hdrOnly && !parsed.hdrType) return false;
        if (freeleechOnly && !release.isFreeleech) return false;
        if (seededOnly && !(release.seeders && release.seeders > 0)) return false;
        return true;
      }),
    [allReleases, resolution, codec, packs, language, hdrOnly, freeleechOnly, seededOnly],
  );
  const failures = result.data?.searchSources.sources.filter((source) => source.error) ?? [];

  const grab = async (release: Release) => {
    setBusy(release.guid);
    try {
      const { data } = await addTorrent({
        variables: {
          input: {
            magnet: release.magnetUri ?? null,
            url: release.magnetUri ? null : release.link,
            libraryId: libraryId ?? null,
            movieId: target?.movieId ?? null,
            showId: target?.showId ?? null,
            episodeId: release.parsed.isSeasonPack ? null : target?.episodeId ?? null,
            albumId: target?.albumId ?? null,
            trackId: target?.trackId ?? null,
            audiobookId: target?.audiobookId ?? null,
            chapterId: target?.chapterId ?? null,
            season: release.parsed.isSeasonPack ? release.parsed.season ?? season ?? null : null,
            sourceIndexerId: release.sourceId ?? null,
            sourceUrl: release.details ?? null,
          },
        },
      });
      assertSuccess(data?.addTorrent, "Could not start the download");
      toast.success(`Downloading ${data?.addTorrent.torrent?.name ?? release.title}`);
      onOpenChange(false);
    } catch (error) {
      toast.danger(errorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const columns = useMemo<Array<DataTableColumn<Release>>>(
    () => [
      {
        id: "title",
        header: "Release",
        wrap: true,
        cell: (release) => (
          <span className="min-w-0">
            <span className="block break-words text-body-sm text-foreground">{release.title}</span>
            <span className="mt-1 flex flex-wrap items-center gap-1">
              <span className="text-label-sm text-muted">{[release.sourceName, formatRelative(release.publishDate)].filter(Boolean).join(" · ")}</span>
              <ReleaseTags release={release} />
            </span>
          </span>
        ),
      },
      {
        id: "match",
        header: "Profile",
        size: 130,
        cell: (release) =>
          release.profileMatch ? (
            release.rejectReasons.length > 0 ? (
              <Tooltip delay={200} closeDelay={0}>
                <Button variant="ghost" size="sm" data-focusable aria-label={`Why ${release.profileMatch}: ${release.rejectReasons.join(", ")}`} className="h-auto min-h-0 cursor-help rounded-pill border-0 bg-transparent p-0 shadow-none backdrop-blur-none">
                  <StatusChip status={MATCH[release.profileMatch] ?? MATCH.suboptimal!} />
                </Button>
                <Tooltip.Content placement="top">
                  <span className="block max-w-64 text-label-sm">{release.rejectReasons.join(" · ")}</span>
                </Tooltip.Content>
              </Tooltip>
            ) : (
              <StatusChip status={MATCH[release.profileMatch] ?? MATCH.suboptimal!} />
            )
          ) : (
            <span className="text-muted">—</span>
          ),
      },
      { id: "size", header: "Size", size: 100, align: "end", numeric: true, cell: (release) => release.sizeFormatted ?? formatBytes(release.size) },
      { id: "seeders", header: "Seeds", size: 80, align: "end", numeric: true, cell: (release) => <span className={release.seeders ? "text-success" : "text-muted"}>{release.seeders ?? "—"}</span> },
      { id: "leechers", header: "Peers", size: 80, align: "end", numeric: true, hideBelow: "md", cell: (release) => release.leechers ?? "—" },
      {
        id: "grab",
        header: "",
        size: 110,
        align: "end",
        cell: (release) => (
          <Button size="sm" variant="primary" isPending={busy === release.guid} onPress={() => void grab(release)} isDisabled={!release.magnetUri && !release.link}>
            <IconDownload size={14} /> Grab
          </Button>
        ),
      },
    ],
    [busy],
  );

  return (
    <Dialog isOpen={isOpen} onOpenChange={onOpenChange} title="Find a release" size="wide">
      <div className="flex flex-col gap-4">
        <form
          className="flex gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            void search({ variables: { input: { ...variables.input, query } } });
          }}
        >
          <SearchField aria-label="Search releases" value={query} onChange={setQuery} className="flex-1">
            <SearchField.Group>
              <SearchField.SearchIcon>
                <IconSearch size={16} />
              </SearchField.SearchIcon>
              <SearchField.Input placeholder="Search releases" />
              <SearchField.ClearButton>
                <IconX size={14} />
              </SearchField.ClearButton>
            </SearchField.Group>
          </SearchField>
          <Button type="submit" variant="secondary" isPending={result.loading}>
            Search
          </Button>
        </form>
        {allReleases.length > 0 ? (
          <div className="flex flex-wrap items-center gap-2">
            <GlassSegmented<Resolution> ariaLabel="Resolution" size="sm" value={resolution} onChange={setResolution} items={[{ key: "any", label: "Any" }, { key: "2160p", label: "2160p" }, { key: "1080p", label: "1080p" }, { key: "720p", label: "720p" }, { key: "sd", label: "SD" }]} />
            <GlassSegmented<Codec> ariaLabel="Codec" size="sm" value={codec} onChange={setCodec} items={[{ key: "any", label: "Any codec" }, { key: "hevc", label: "HEVC" }, { key: "h264", label: "H.264" }, { key: "av1", label: "AV1" }]} />
            <GlassSegmented<Packs> ariaLabel="Season packs" size="sm" value={packs} onChange={setPacks} items={[{ key: "any", label: "All" }, { key: "packs", label: "Season packs" }, { key: "episodes", label: "Single" }]} />
            {languages.length > 0 ? (
              <Select aria-label="Language" selectedKey={language} onSelectionChange={(key) => key !== null && setLanguage(String(key))} className="w-40" variant="secondary">
                <Select.Trigger className="h-8 text-label">
                  <Select.Value />
                  <Select.Indicator />
                </Select.Trigger>
                <Select.Popover>
                  <ListBox>
                    <ListBoxItem id="any" textValue="Any language">
                      Any language
                    </ListBoxItem>
                    {languages.map((code) => (
                      <ListBoxItem key={code} id={code} textValue={languageName(code)}>
                        {languageName(code)}
                      </ListBoxItem>
                    ))}
                  </ListBox>
                </Select.Popover>
              </Select>
            ) : null}
            <Button size="sm" variant={hdrOnly ? "primary" : "secondary"} onPress={() => setHdrOnly((value) => !value)} aria-pressed={hdrOnly}>
              HDR
            </Button>
            <Button size="sm" variant={freeleechOnly ? "primary" : "secondary"} onPress={() => setFreeleechOnly((value) => !value)} aria-pressed={freeleechOnly}>
              Freeleech
            </Button>
            <Button size="sm" variant={seededOnly ? "primary" : "secondary"} onPress={() => setSeededOnly((value) => !value)} aria-pressed={seededOnly}>
              Seeded
            </Button>
            <span className="text-numeric ml-auto text-label-sm text-muted">{releases.length} of {allReleases.length}</span>
          </div>
        ) : null}
        {sources.data && sources.data.sources.edges.length === 0 ? (
          <EmptyState compact icon={IconSearch} title="No sources configured" description="Add a torrent indexer under Settings → Sources to search for releases." />
        ) : result.loading ? (
          <div className="grid h-48 place-items-center">
            <Spinner size={28} />
          </div>
        ) : result.error ? (
          <ErrorState error={result.error} onRetry={() => void search({ variables })} />
        ) : (
          <>
            {failures.length ? <p className="text-label-sm text-warning">{failures.map((source) => `${source.sourceName}: ${source.error}`).join(" · ")}</p> : null}
            <DataTable<Release>
              frame={false}
              columns={columns}
              rows={releases}
              getRowId={(release) => release.guid}
              density="compact"
              noun="releases"
              emptyState={<EmptyState compact icon={IconSearch} title={allReleases.length ? "Nothing matches these filters" : "No releases found"} description={allReleases.length ? "Loosen a filter to see more." : "Try a shorter title or search without the year."} />}
            />
            {result.data ? <p className="text-label-sm text-muted">{result.data.searchSources.totalReleases} releases from {result.data.searchSources.sourcesSearched} sources in {result.data.searchSources.totalElapsedMs} ms</p> : null}
          </>
        )}
      </div>
    </Dialog>
  );
}

/** Quality tags the backend parsed out of the release title. */
function ReleaseTags({ release }: { release: Release }) {
  const parsed = release.parsed;
  const tags: Array<{ key: string; label: string; tone?: "brand" | "info" }> = [];
  if (parsed.resolution) tags.push({ key: "resolution", label: parsed.resolution });
  if (parsed.sourceType) tags.push({ key: "source", label: parsed.sourceType });
  if (parsed.codec) tags.push({ key: "codec", label: parsed.codec.toUpperCase() });
  if (parsed.hdrType) tags.push({ key: "hdr", label: parsed.hdrType.toUpperCase() });
  if (parsed.audio) tags.push({ key: "audio", label: parsed.audio.toUpperCase() });
  for (const code of parsed.languages) tags.push({ key: `lang-${code}`, label: languageName(code) });
  if (parsed.isSeasonPack) tags.push({ key: "pack", label: parsed.season !== null && parsed.season !== undefined ? `Season ${parsed.season} pack` : "Season pack", tone: "info" });
  if (parsed.isProper) tags.push({ key: "proper", label: "PROPER", tone: "brand" });
  if (parsed.isRepack) tags.push({ key: "repack", label: "REPACK", tone: "brand" });
  if (parsed.releaseGroup) tags.push({ key: "group", label: parsed.releaseGroup });
  if (release.isFreeleech) tags.push({ key: "freeleech", label: "Freeleech", tone: "info" });

  return (
    <>
      {tags.map((tag) => (
        <span
          key={tag.key}
          className={cn(
            "rounded-pill px-1.5 py-0.5 text-label-sm",
            tag.tone === "brand" ? "bg-brand-soft text-foreground" : tag.tone === "info" ? "bg-info-soft text-info" : "bg-surface-tertiary text-muted",
          )}
        >
          {tag.label}
        </span>
      ))}
    </>
  );
}
