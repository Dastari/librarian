import { useMutation } from "@apollo/client/react";
import { useSourceSearch } from "./useSourceSearch";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { SearchField } from "glass-ux/input";
import { Select } from "glass-ux/select";
import { Tooltip } from "glass-ux/tooltip";
import { toast } from "glass-ux/toast";
import { IconDownload, IconSearch } from "@tabler/icons-react";
import { Badge, type BadgeTone } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, NumericCell, TextCell } from "glass-ux/data-table";
import { LevelGauge, type GaugeLevel } from "glass-ux/level-gauge";
import { createContext, useContext, useEffect, useMemo, useState } from "react";

import { Button, Dialog, EmptyState, ErrorState, GlassSegmented, Spinner } from "@/components/ui";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { AddTorrentDocument, HasEnabledSourcesDocument, type SearchSourcesQuery } from "@/graphql/generated/graphql";
import { formatBytes, formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { languageName } from "@/lib/languages";
import { cn } from "@/lib/utils";

type Release = SearchSourcesQuery["searchSources"]["sources"][number]["releases"][number];
const releaseId = (release: Release) => release.stableId;
const ReleaseActionsContext = createContext<{ busy: string | null; grab: (release: Release) => Promise<void> } | null>(null);

function GrabReleaseCell({ release }: { release: Release }) {
  const actions = useContext(ReleaseActionsContext)!;
  return (
    <Button size="sm" variant="primary" isPending={actions.busy === releaseId(release)} onPress={() => void actions.grab(release)} isDisabled={!release.magnetUri && !release.link}>
      <IconDownload size={14} /> Grab
    </Button>
  );
}

type Resolution = "any" | "2160p" | "1080p" | "720p" | "sd";
type Codec = "any" | "hevc" | "h264" | "av1";
type Packs = "any" | "packs" | "episodes";

/** The backend's verdict for the resolved quality profile. */
const MATCH: Record<string, { label: string; tone: BadgeTone }> = {
  optimal: { label: "Optimal", tone: "success" },
  suboptimal: { label: "Suboptimal", tone: "warning" },
  rejected: { label: "Rejected", tone: "danger" },
};
/** Seeders as swarm health: whether a grab is likely to finish. */
type Health = "dead" | "weak" | "healthy";
const HEALTH: readonly GaugeLevel<Health>[] = [
  { value: "dead", label: "No seeders", tone: "danger" },
  { value: "weak", label: "Few seeders", tone: "warning" },
  { value: "healthy", label: "Well seeded", tone: "success" },
];
const health = (seeders: number | null | undefined): Health => (!seeders ? "dead" : seeders < 5 ? "weak" : "healthy");
const ReleaseGlyph = ENTITY_KINDS.torrent.icon;

function ProfileMatchCell({ release }: { release: Release }) {
  if (!release.profileMatch) return <TextCell muted>—</TextCell>;
  const match = MATCH[release.profileMatch] ?? MATCH.suboptimal!;
  const badge = <Badge size="sm" marker="dot" tone={match.tone}>{match.label}</Badge>;
  if (!release.rejectReasons.length) return badge;
  return (
    <Tooltip content={release.rejectReasons.join(" · ")}>
      <Button variant="ghost" size="sm" data-focusable aria-label={`Why ${release.profileMatch}: ${release.rejectReasons.join(", ")}`} className="h-auto min-h-0 cursor-help rounded-pill border-0 bg-transparent p-0 shadow-none backdrop-blur-none">{badge}</Button>
    </Tooltip>
  );
}

const column = createDataTableColumns<Release>();
const columns = column.columns([
  column.accessor("title", {
    header: "Release", width: 30, priority: 1, cardRole: "title",
    cell: ({ row }) => <IdentityCell icon={<ReleaseGlyph />} title={row.original.title} subtitle={[row.original.sourceName, formatRelative(row.original.publishDate)].filter(Boolean).join(" · ")} />,
  }),
  column.display({ id: "tags", header: "Quality", width: 18, priority: 2, cell: ({ row }) => <span className="flex min-w-0 flex-wrap items-center gap-1"><ReleaseTags release={row.original} /></span> }),
  column.accessor((release) => release.profileMatch ?? "", { id: "match", header: "Profile", width: 9, priority: 2, cardRole: "status", cell: ({ row }) => <ProfileMatchCell release={row.original} /> }),
  column.accessor("size", { header: "Size", width: 7, priority: 2, align: "end", grow: false, cell: ({ row }) => <TextCell muted>{row.original.sizeFormatted ?? formatBytes(row.original.size)}</TextCell> }),
  column.accessor((release) => release.seeders ?? 0, {
    id: "seeders", header: "Seeds", width: 9, priority: 2,
    cell: ({ row }) => <span className="flex items-center gap-2"><span className="text-numeric w-8 text-end tabular-nums">{row.original.seeders ?? "—"}</span><LevelGauge label="Swarm health" levels={HEALTH} value={health(row.original.seeders)} /></span>,
  }),
  column.accessor((release) => release.leechers ?? 0, { id: "leechers", header: "Peers", width: 5, priority: 4, align: "end", grow: false, cell: ({ row }) => (row.original.leechers === null || row.original.leechers === undefined ? <TextCell muted>—</TextCell> : <NumericCell value={row.original.leechers} />) }),
  column.display({ id: "grab", header: "", width: 7, priority: 1, align: "end", grow: false, cell: ({ row }) => <GrabReleaseCell release={row.original} /> }),
]);
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
  const { search, result, loadMore, retrySource, retryContinuation, revision, hasMore, queryKey } = useSourceSearch();
  const [addTorrent] = useMutation(AddTorrentDocument);
  const [busy, setBusy] = useState<string | null>(null);
  const [resolution, setResolution] = useState<Resolution>("any");
  const [codec, setCodec] = useState<Codec>("any");
  const [packs, setPacks] = useState<Packs>("any");
  const [language, setLanguage] = useState<string>("any");
  const [hdrOnly, setHdrOnly] = useState(false);
  const [freeleechOnly, setFreeleechOnly] = useState(false);
  const [seededOnly, setSeededOnly] = useState(true);
  const sourceCapability = useQuery(HasEnabledSourcesDocument, { skip: !isOpen });

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
        // The response count describes this batch, not an external total.
        // Local filters apply to accumulated returned releases only.
        limit: null,
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
  const retryableSource = failures.find((source) => source.pageInfo.retryCursor);
  const paginationError = result.moreError ?? (!hasMore && retryableSource ? new Error(retryableSource.error ?? "Source page failed") : undefined);
  const retryPage = () => retryableSource && !result.moreError ? retrySource(retryableSource.sourceId) : retryContinuation();

  const grab = async (release: Release) => {
    setBusy(releaseId(release));
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

  return (
    <ReleaseActionsContext value={{ busy, grab }}>
    <Dialog isOpen={isOpen} onOpenChange={onOpenChange} title="Find a release" size="wide">
      <div className="flex flex-col gap-4">
        <form
          className="flex min-w-0 gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            void search({ variables: { input: { ...variables.input, query } } });
          }}
        >
          <SearchField aria-label="Search releases" value={query} onValueChange={setQuery} placeholder="Search releases" className="min-w-0 flex-1" />
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
              <Select aria-label="Language" value={language} onValueChange={(value) => setLanguage(value ?? "any")} size="sm" className="w-40" options={[{ value: "any", label: "Any language" }, ...languages.map((code) => ({ value: code, label: languageName(code) }))]} />
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
        {sourceCapability.data?.hasEnabledSources === false ? (
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
            {result.data ? <p className="text-label-sm text-muted">Source results may be partial. Filters apply to returned releases.</p> : null}
            <DataTable<Release>
              id="librarian.release-search"
              label="Releases"
              columns={columns}
              rows={releases}
              queryKey={queryKey}
              infinite={{ revision, hasMore, onLoadMore: loadMore, loadingMore: result.loadingMore, error: paginationError?.message }}
              onRetry={() => void retryPage()}
              getRowId={releaseId}
              getRowLabel={(release) => release.title}
              density="compact"
              noun={{ one: "release", other: "releases" }}
              emptyMessage={<span className="flex flex-col items-center gap-1"><span>{allReleases.length ? "Nothing matches these filters" : "No releases found"}</span><span className="text-label-sm text-muted">{allReleases.length ? "Loosen a filter to see more." : "Try a shorter title or search without the year."}</span></span>}
            />
            {result.data ? <p className="text-label-sm text-muted">{allReleases.length} returned releases from {result.data.searchSources.sources.length} sources · latest response {result.data.searchSources.totalElapsedMs} ms</p> : null}
          </>
        )}
      </div>
    </Dialog>
    </ReleaseActionsContext>
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
