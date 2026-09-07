import { useMemo, useState, useEffect, useCallback, useRef } from "react";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Button } from "@heroui/button";
import { Chip } from "@heroui/chip";
import { Spinner } from "@heroui/spinner";
import { Select, SelectItem } from "@heroui/select";
import { Card, CardBody } from "@heroui/card";
import { Divider } from "@heroui/divider";
import { Progress } from "@heroui/progress";
import { Tooltip } from "@heroui/tooltip";
import { Checkbox } from "@heroui/checkbox";
import { Image } from "@heroui/image";
import { addToast } from "@heroui/toast";
import {
  IconCheck,
  IconX,
  IconMovie,
  IconDeviceTv,
  IconMusic,
  IconHeadphones,
  IconFile,
  IconCircleCheck,
  IconAlertTriangle,
  IconArrowsShuffle,
  IconBan,
  IconUnlink,
} from "@tabler/icons-react";
import {
  CreateUnmatchedMediaFileFromTorrentDocument,
  TorrentByInfoHashWithFilesDocument,
  TorrentFindMatchForMediaFileRuntimeDocument,
  TorrentMatchCandidateArtworkRuntimeDocument,
  TorrentMatchLibrariesRuntimeDocument,
  TorrentMatchMediaFilesByPathsRuntimeDocument,
  TorrentUnmatchMediaFileRuntimeDocument,
  type CreateUnmatchedMediaFileFromTorrentMutation,
  type CreateUnmatchedMediaFileFromTorrentMutationVariables,
  type TorrentByInfoHashWithFilesQuery,
  type TorrentByInfoHashWithFilesQueryVariables,
  type TorrentFindMatchForMediaFileRuntimeMutation,
  type TorrentMatchCandidateArtworkRuntimeQuery,
  type TorrentMatchLibrariesRuntimeQuery,
  type TorrentMatchMediaFilesByPathsRuntimeQuery,
  type TorrentUnmatchMediaFileRuntimeMutation,
} from "../../lib/graphql/generated/graphql";
import { apolloClient, useMutation, useQuery } from "../../lib/graphql/client";
import { sanitizeError, formatBytes } from "../../lib/format";

// ============================================================================
// Constants
// ============================================================================

const ALL_LIBRARIES_KEY = "__all__";
const ALL_LIBRARY_TYPES_KEY = "__all_types__";
const DECLINE_KEY = "__decline__";
const UNMATCH_KEY = "__unmatch__";
const UNMATCHED_LIBRARY_ID = "__torrent_unmatched__";

// ============================================================================
// Types
// ============================================================================

type LibraryNode =
  TorrentMatchLibrariesRuntimeQuery["libraries"]["edges"][number]["node"];

type TorrentFileNode =
  TorrentByInfoHashWithFilesQuery["torrents"]["edges"][number]["node"]["files"]["edges"][number]["node"];

type MediaLookupNode =
  TorrentMatchMediaFilesByPathsRuntimeQuery["mediaFiles"]["edges"][number]["node"];

type MatchCandidate =
  TorrentFindMatchForMediaFileRuntimeMutation["matchMediaFile"]["candidates"][number];

interface MatchRowState {
  rowId: string;
  fileIndex: number | null;
  filePath: string;
  fileSize: number;
  mediaFileId: string | null;
  existingMatchType: string | null;
  existingMatchId: string | null;
  candidates: MatchCandidate[];
  selectedKey: string;
  status:
    | "idle"
    | "finding"
    | "ready"
    | "applied"
    | "unmatched"
    | "declined"
    | "error";
  error: string | null;
}

export interface MediaFilesMatchDialogProps {
  isOpen: boolean;
  onClose: () => void;
  torrentInfoHash?: string | null;
  mediaFiles?: MediaFileMatchInput[] | null;
  contextName?: string | null;
  initialFileIndex?: number | null;
  onApplied?: () => void;
}

export interface MediaFileMatchInput {
  RowId?: string | null;
  fileIndex?: number | null;
  filePath: string;
  fileSize: number;
  mediaFileId?: string | null;
  episodeId?: string | null;
  movieId?: string | null;
  trackId?: string | null;
  chapterId?: string | null;
}

// ============================================================================
// Utility Functions
// ============================================================================

function normalizePathForLookup(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+/g, "/").toLowerCase();
}

function isAbsolutePath(path: string): boolean {
  return (
    path.startsWith("/") ||
    path.startsWith("\\\\") ||
    /^[A-Za-z]:[\\/]/.test(path)
  );
}

function joinPath(base: string, segment: string): string {
  const normalizedBase = base.replace(/\\/g, "/").replace(/\/+$/, "");
  const normalizedSegment = segment.replace(/\\/g, "/").replace(/^\/+/, "");
  return `${normalizedBase}/${normalizedSegment}`;
}

function buildPathCandidates(
  filePath: string,
  savePath?: string | null,
  torrentName?: string | null,
): string[] {
  const candidates = new Set<string>();
  const normalizedFilePath = filePath.replace(/\\/g, "/");
  candidates.add(filePath);
  candidates.add(normalizedFilePath);

  if (savePath && !isAbsolutePath(normalizedFilePath)) {
    candidates.add(joinPath(savePath, normalizedFilePath));
    if (torrentName) {
      candidates.add(
        joinPath(joinPath(savePath, torrentName), normalizedFilePath),
      );
    }
  }

  return Array.from(candidates);
}

function getRelativePath(
  filePath: string,
  savePath?: string | null,
  torrentName?: string | null,
): string {
  const normalized = filePath.replace(/\\/g, "/");
  if (!isAbsolutePath(normalized)) {
    return normalized;
  }
  if (!savePath) {
    return normalized.split("/").pop() ?? normalized;
  }
  const savePrefix = `${savePath.replace(/\\/g, "/").replace(/\/+$/, "")}/`;
  if (torrentName) {
    const withTorrentPrefix = `${savePrefix}${torrentName}/`;
    if (normalized.startsWith(withTorrentPrefix)) {
      return normalized.slice(withTorrentPrefix.length);
    }
  }
  if (normalized.startsWith(savePrefix)) {
    return normalized.slice(savePrefix.length);
  }
  return normalized.split("/").pop() ?? normalized;
}

function isMatchableMediaFile(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return new Set([
    "mkv",
    "mp4",
    "avi",
    "m4v",
    "mov",
    "wmv",
    "flv",
    "webm",
    "mpeg",
    "mpg",
    "ts",
    "m2ts",
    "mp3",
    "flac",
    "m4a",
    "m4b",
    "aac",
    "ogg",
    "opus",
    "wav",
    "wma",
    "aiff",
    "alac",
    "ape",
    "dsf",
    "dff",
  ]).has(ext);
}

function normalizeLibraryType(raw: string | null | undefined): string {
  const type = (raw ?? "").trim().toUpperCase();
  if (type === "SHOWS" || type === "TVSHOW" || type === "TVSHOWS") return "TV";
  if (type === "MOVIE") return "MOVIES";
  if (type === "AUDIOBOOK") return "AUDIOBOOKS";
  return type;
}

function formatLibraryTypeLabel(type: string): string {
  if (type === ALL_LIBRARY_TYPES_KEY) return "All";
  switch (type) {
    case "MOVIES":
      return "Movies";
    case "TV":
      return "TV";
    case "MUSIC":
      return "Music";
    case "AUDIOBOOKS":
      return "Audiobooks";
    default:
      return type
        .toLowerCase()
        .replace(/_/g, " ")
        .replace(/\b\w/g, (m) => m.toUpperCase());
  }
}

function mergeCandidatesByScore(input: MatchCandidate[]): MatchCandidate[] {
  const merged = new Map<string, MatchCandidate>();
  for (const candidate of input) {
    const key = `${candidate.targetType}:${candidate.targetId}`;
    const current = merged.get(key);
    if (!current || candidate.score > current.score) {
      merged.set(key, candidate);
    }
  }
  return Array.from(merged.values())
    .sort((a, b) => b.score - a.score)
    .slice(0, 10);
}

function getFileName(filePath: string): string {
  const normalized = filePath.replace(/\\/g, "/");
  return normalized.split("/").pop() ?? normalized;
}

function getTypeIcon(targetType: string, size = 24) {
  switch (targetType) {
    case "Movie":
      return <IconMovie size={size} className="text-purple-400" />;
    case "Episode":
      return <IconDeviceTv size={size} className="text-blue-400" />;
    case "Track":
      return <IconMusic size={size} className="text-green-400" />;
    case "Chapter":
      return <IconHeadphones size={size} className="text-amber-400" />;
    default:
      return <IconFile size={size} className="text-default-400" />;
  }
}

function getTypeGradient(targetType: string): string {
  switch (targetType) {
    case "Movie":
      return "from-violet-900/60 via-purple-800/40 to-fuchsia-900/60";
    case "Episode":
      return "from-blue-900/60 via-indigo-800/40 to-cyan-900/60";
    case "Track":
      return "from-emerald-900/60 via-green-800/40 to-teal-900/60";
    case "Chapter":
      return "from-amber-900/60 via-orange-800/40 to-yellow-900/60";
    default:
      return "from-slate-800/60 via-gray-700/40 to-zinc-800/60";
  }
}

function getStatusColor(
  status: MatchRowState["status"],
): "success" | "danger" | "primary" | "warning" | "default" {
  switch (status) {
    case "applied":
      return "success";
    case "ready":
      return "success";
    case "error":
      return "danger";
    case "finding":
      return "primary";
    case "unmatched":
      return "warning";
    default:
      return "default";
  }
}

function getStatusLabel(status: MatchRowState["status"]): string {
  switch (status) {
    case "idle":
      return "Pending";
    case "finding":
      return "Searching...";
    case "ready":
      return "Match Found";
    case "applied":
      return "Applied";
    case "unmatched":
      return "Unmatched";
    case "declined":
      return "Skipped";
    case "error":
      return "Error";
  }
}

// ============================================================================
// Sub-Components
// ============================================================================

/** Card showing the currently selected match candidate */
function MatchCandidateCard({
  candidate,
  allCandidates,
  artworkUrl,
  selectedKey,
  onSelect,
  status,
  error,
}: {
  candidate: MatchCandidate | null;
  allCandidates: MatchCandidate[];
  artworkUrl?: string | null;
  selectedKey: string;
  onSelect: (key: string) => void;
  status: MatchRowState["status"];
  error: string | null;
}) {
  if (status === "finding") {
    return (
      <Card className="bg-content2 border border-default-200 h-full">
        <CardBody className="flex items-center justify-center gap-2 py-8">
          <Spinner size="sm" />
          <span className="text-sm text-default-500">
            Finding best match...
          </span>
        </CardBody>
      </Card>
    );
  }

  if (status === "idle") {
    return (
      <Card className="bg-content2 border border-dashed border-default-300 h-full">
        <CardBody className="flex items-center justify-center py-8">
          <div className="text-center space-y-1">
            <IconArrowsShuffle size={28} className="text-default-400 mx-auto" />
            <p className="text-sm text-default-500">Awaiting match</p>
          </div>
        </CardBody>
      </Card>
    );
  }

  if (status === "error") {
    return (
      <Card className="bg-content2 border border-danger-200 h-full">
        <CardBody className="flex items-center justify-center py-8">
          <div className="text-center space-y-1">
            <IconAlertTriangle size={28} className="text-danger-400 mx-auto" />
            <p className="text-xs text-danger-400">{error || "Match failed"}</p>
          </div>
        </CardBody>
      </Card>
    );
  }

  const isDeclined = selectedKey === DECLINE_KEY;
  const isUnmatch = selectedKey === UNMATCH_KEY;

  if (isDeclined || isUnmatch || !candidate) {
    return (
      <Card className="bg-content2 border border-default-200 h-full">
        <CardBody className="space-y-3">
          <div className="flex items-center justify-center py-4">
            <div className="text-center space-y-1">
              {isUnmatch ? (
                <>
                  <IconUnlink size={28} className="text-warning-400 mx-auto" />
                  <p className="text-sm text-warning-500">
                    Will unmatch current link
                  </p>
                </>
              ) : (
                <>
                  <IconBan size={28} className="text-default-400 mx-auto" />
                  <p className="text-sm text-default-500">No match selected</p>
                </>
              )}
            </div>
          </div>
          {allCandidates.length > 0 && (
            <Select
              label="Select a match"
              size="sm"
              selectedKeys={[selectedKey]}
              onSelectionChange={(keys) => {
                const key = Array.from(keys)[0]?.toString();
                if (key) onSelect(key);
              }}
              classNames={{
                trigger: "bg-content1",
              }}
            >
              {[
                <SelectItem key={DECLINE_KEY} textValue="No match / Skip">
                  No match / Skip
                </SelectItem>,
                <SelectItem key={UNMATCH_KEY} textValue="Unmatch current">
                  Unmatch current
                </SelectItem>,
                ...allCandidates.map((c) => {
                  const key = `${c.targetType}:${c.targetId}`;
                  const label =
                    c.targetName ?? `${c.targetType} ${c.targetId.slice(0, 8)}`;
                  const textVal = `${label} (${(c.score * 100).toFixed(0)}%)`;
                  return (
                    <SelectItem key={key} textValue={textVal}>
                      {textVal}
                    </SelectItem>
                  );
                }),
              ]}
            </Select>
          )}
        </CardBody>
      </Card>
    );
  }

  // Show the matched entity card
  const gradient = getTypeGradient(candidate.targetType);
  const displayName =
    candidate.targetName ??
    `${candidate.targetType} ${candidate.targetId.slice(0, 8)}...`;
  const confidence = Math.round(candidate.score * 100);
  const isSelected = selectedKey !== DECLINE_KEY && selectedKey !== UNMATCH_KEY;

  return (
    <Card className="bg-content2 border border-default-200 h-full overflow-hidden">
      <CardBody className="p-0 space-y-0">
        {/* Entity header with gradient */}
        <div
          className={`bg-gradient-to-r ${gradient} px-4 py-3 flex items-center gap-3`}
        >
          {artworkUrl ? (
            <Image
              src={artworkUrl}
              alt={displayName}
              removeWrapper
              className="w-12 h-16 rounded-md object-cover border border-white/20 shrink-0"
            />
          ) : (
            getTypeIcon(candidate.targetType, 28)
          )}
          <div className="min-w-0 flex-1">
            <p className="text-sm font-bold text-white truncate">
              {displayName}
            </p>
            <div className="flex items-center gap-2 mt-0.5">
              <Chip
                size="sm"
                variant="flat"
                color={confidence >= 70 ? "success" : "warning"}
                className="h-5"
              >
                {confidence}% match
              </Chip>
              {candidate.wanted && (
                <Chip size="sm" variant="flat" color="primary" className="h-5">
                  Wanted
                </Chip>
              )}
              <span className="text-xs text-white/60">
                {candidate.targetType}
              </span>
            </div>
          </div>
          <Checkbox
            isSelected={isSelected}
            onValueChange={(value) => {
              if (value) {
                onSelect(`${candidate.targetType}:${candidate.targetId}`);
                return;
              }
              onSelect(DECLINE_KEY);
            }}
            color="success"
            size="sm"
            icon={<IconCheck size={12} stroke={3} />}
            classNames={{
              wrapper: "border-white/50 bg-white/10",
              label: "text-white/90 text-xs",
            }}
          >
            Match
          </Checkbox>
          {status === "applied" && (
            <IconCircleCheck size={20} className="text-green-400 shrink-0" />
          )}
        </div>

        {/* Candidate selector */}
        {allCandidates.length > 1 && (
          <div className="px-3 py-2">
            <Select
              label="Alternative matches"
              size="sm"
              selectedKeys={[selectedKey]}
              onSelectionChange={(keys) => {
                const key = Array.from(keys)[0]?.toString();
                if (key) onSelect(key);
              }}
              classNames={{
                trigger: "bg-content3",
              }}
            >
              {[
                ...allCandidates.map((c) => {
                  const key = `${c.targetType}:${c.targetId}`;
                  const label =
                    c.targetName ?? `${c.targetType} ${c.targetId.slice(0, 8)}`;
                  const textVal = `${label} (${(c.score * 100).toFixed(0)}%)`;
                  return (
                    <SelectItem key={key} textValue={textVal}>
                      {textVal}
                    </SelectItem>
                  );
                }),
                <SelectItem key={DECLINE_KEY} textValue="No match / Skip">
                  No match / Skip
                </SelectItem>,
                <SelectItem key={UNMATCH_KEY} textValue="Unmatch current">
                  Unmatch current
                </SelectItem>,
              ]}
            </Select>
          </div>
        )}

        <div className="px-4 py-2">
          <p className="text-xs text-default-500">{confidence}% confidence</p>
        </div>
      </CardBody>
    </Card>
  );
}

/** Left side: the filename being matched */
function FileInfoPanel({ row }: { row: MatchRowState }) {
  const fileName = getFileName(row.filePath);

  return (
    <Card className="bg-content2 border border-default-200 h-full">
      <CardBody className="flex flex-col justify-center gap-2 py-4">
        <div className="flex items-center gap-2">
          <IconFile size={20} className="text-default-400 shrink-0" />
          <Tooltip content={row.filePath} delay={500}>
            <p className="text-sm font-medium truncate">{fileName}</p>
          </Tooltip>
        </div>
        <div className="flex items-center gap-2 text-xs text-default-500">
          <span className="tabular-nums">{formatBytes(row.fileSize)}</span>
          {row.fileIndex != null && (
            <>
              <span>•</span>
              <span>File #{row.fileIndex}</span>
            </>
          )}
        </div>
        <div className="mt-1">
          <Chip size="sm" variant="flat" color={getStatusColor(row.status)}>
            {getStatusLabel(row.status)}
          </Chip>
        </div>
      </CardBody>
    </Card>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function MediaFilesMatchDialog({
  isOpen,
  onClose,
  mediaFiles = null,
  contextName = null,
  torrentInfoHash,
  initialFileIndex = null,
  onApplied,
}: MediaFilesMatchDialogProps) {
  const [rows, setRows] = useState<MatchRowState[]>([]);
  const [candidateArtworkByKey, setCandidateArtworkByKey] = useState<
    Record<string, string>
  >({});
  const [isFinding, setIsFinding] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [selectedLibraryType, setSelectedLibraryType] = useState<string>(
    ALL_LIBRARY_TYPES_KEY,
  );
  const [selectedLibraryId, setSelectedLibraryId] =
    useState<string>(ALL_LIBRARIES_KEY);
  const matchScopeKeyRef = useRef<string | null>(null);

  const torrentQueryVariables =
    useMemo<TorrentByInfoHashWithFilesQueryVariables>(
      () => ({
        where: { infoHash: { eq: torrentInfoHash ?? "" } },
        page: { limit: 1, offset: 0 },
      }),
      [torrentInfoHash],
    );

  const { data: torrentData, loading: torrentLoading } = useQuery(
    TorrentByInfoHashWithFilesDocument,
    {
      variables: torrentQueryVariables,
      skip: !isOpen || !torrentInfoHash || (mediaFiles?.length ?? 0) > 0,
      fetchPolicy: "cache-and-network",
      notifyOnNetworkStatusChange: true,
    },
  );
  const { data: librariesData, loading: librariesLoading } =
    useQuery<TorrentMatchLibrariesRuntimeQuery>(
      TorrentMatchLibrariesRuntimeDocument,
      {
        skip: !isOpen,
        fetchPolicy: "cache-and-network",
        notifyOnNetworkStatusChange: true,
      },
    );

  const [createUnmatchedMediaFile] = useMutation<
    CreateUnmatchedMediaFileFromTorrentMutation,
    CreateUnmatchedMediaFileFromTorrentMutationVariables
  >(CreateUnmatchedMediaFileFromTorrentDocument);
  const [findMatch] = useMutation<TorrentFindMatchForMediaFileRuntimeMutation>(
    TorrentFindMatchForMediaFileRuntimeDocument,
  );
  const [unmatchMediaFile] =
    useMutation<TorrentUnmatchMediaFileRuntimeMutation>(
      TorrentUnmatchMediaFileRuntimeDocument,
  );

  const torrent = torrentData?.torrents?.edges?.[0]?.node ?? null;
  const torrentFiles = useMemo<TorrentFileNode[]>(() => {
    const files =
      torrent?.files?.edges?.map((e) => e.node).filter((f) =>
        isMatchableMediaFile(f.filePath),
      ) ?? [];
    if (initialFileIndex == null) return files;
    return files.filter((f) => f.fileIndex === initialFileIndex);
  }, [torrent?.files?.edges, initialFileIndex]);
  const inputMediaFiles = useMemo<MediaFileMatchInput[]>(() => {
    if (!mediaFiles || mediaFiles.length === 0) return [];
    const files = mediaFiles.filter((f) => isMatchableMediaFile(f.filePath));
    if (initialFileIndex == null) return files;
    return files.filter((f) => f.fileIndex === initialFileIndex);
  }, [mediaFiles, initialFileIndex]);
  const sourceFiles = useMemo<MediaFileMatchInput[]>(() => {
    if (inputMediaFiles.length > 0) return inputMediaFiles;
    return torrentFiles.map((f) => ({
      RowId: `torrent:${f.fileIndex}`,
      fileIndex: f.fileIndex,
      filePath: f.filePath,
      fileSize: f.fileSize,
    }));
  }, [inputMediaFiles, torrentFiles]);

  const allLibraries = useMemo<LibraryNode[]>(
    () => librariesData?.libraries?.edges?.map((e) => e.node) ?? [],
    [librariesData?.libraries?.edges],
  );

  const availableLibraryTypes = useMemo(
    () =>
      Array.from(
        new Set(
          allLibraries
            .map((l) => normalizeLibraryType(l.libraryType))
            .filter(Boolean),
        ),
      ).sort(),
    [allLibraries],
  );

  const librariesByType = useMemo(() => {
    if (selectedLibraryType === ALL_LIBRARY_TYPES_KEY) {
      return allLibraries;
    }
    return allLibraries.filter(
      (l) => normalizeLibraryType(l.libraryType) === selectedLibraryType,
    );
  }, [allLibraries, selectedLibraryType]);

  useEffect(() => {
    if (!isOpen) {
      matchScopeKeyRef.current = null;
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    if (
      selectedLibraryType !== ALL_LIBRARY_TYPES_KEY &&
      !availableLibraryTypes.includes(selectedLibraryType)
    ) {
      setSelectedLibraryType(ALL_LIBRARY_TYPES_KEY);
    }
  }, [isOpen, availableLibraryTypes, selectedLibraryType]);

  useEffect(() => {
    if (!isOpen) return;
    setSelectedLibraryId(ALL_LIBRARIES_KEY);
  }, [isOpen, selectedLibraryType]);

  const lookupMediaByPaths = useCallback(async (paths: string[]) => {
    const result = await apolloClient.query<TorrentMatchMediaFilesByPathsRuntimeQuery>({
      query: TorrentMatchMediaFilesByPathsRuntimeDocument,
      variables: { paths: paths },
      fetchPolicy: "network-only",
    });
    const map = new Map<string, MediaLookupNode>();
    const edges = result.data?.mediaFiles?.edges ?? [];
    for (const edge of edges) {
      map.set(normalizePathForLookup(edge.node.path), edge.node);
    }
    return map;
  }, []);

  const hydrateRows = useCallback(async () => {
    if (!isOpen || sourceFiles.length === 0) {
      setRows([]);
      setCandidateArtworkByKey({});
      return;
    }
    try {
      const savePath = torrent?.savePath ?? null;
      const sourceName = contextName ?? torrent?.name ?? null;
      const allPathCandidates = Array.from(
        new Set(
          sourceFiles.flatMap((f) =>
            buildPathCandidates(f.filePath, savePath, sourceName),
          ),
        ),
      );

      const mediaByPath = await lookupMediaByPaths(allPathCandidates);

      const nextRows = sourceFiles.map((f, idx) => {
        const rowId = f.RowId ?? `row:${f.fileIndex ?? idx}`;
        const candidates = buildPathCandidates(
          f.filePath,
          savePath,
          sourceName,
        );
        let media: MediaLookupNode | null =
          f.mediaFileId != null
            ? ({
                id: f.mediaFileId,
                path: f.filePath,
                episodeId: f.episodeId ?? null,
                movieId: f.movieId ?? null,
                trackId: f.trackId ?? null,
                chapterId: f.chapterId ?? null,
              } as MediaLookupNode)
            : null;
        if (!media) {
          for (const candidate of candidates) {
            media = mediaByPath.get(normalizePathForLookup(candidate)) ?? null;
            if (media) break;
          }
        }
        const existingMatchType = media?.episodeId
          ? "Episode"
          : media?.movieId
            ? "Movie"
            : media?.trackId
              ? "Track"
              : media?.chapterId
                ? "Chapter"
                : null;
        const existingMatchId =
          media?.episodeId ??
          media?.movieId ??
          media?.trackId ??
          media?.chapterId ??
          null;

        return {
          rowId,
          fileIndex: f.fileIndex ?? null,
          filePath: f.filePath,
          fileSize: f.fileSize,
          mediaFileId: f.mediaFileId ?? media?.id ?? null,
          existingMatchType,
          existingMatchId,
          candidates: [],
          selectedKey: DECLINE_KEY,
          status: "idle" as const,
          error: null,
        };
      });

      setRows(nextRows);
      setCandidateArtworkByKey({});
    } catch (error) {
      addToast({
        title: "Failed to load match rows",
        description: sanitizeError(error),
        color: "danger",
      });
      setRows([]);
      setCandidateArtworkByKey({});
    }
  }, [
    isOpen,
    sourceFiles,
    lookupMediaByPaths,
    torrent?.savePath,
    torrent?.name,
    contextName,
  ]);

  useEffect(() => {
    void hydrateRows();
  }, [hydrateRows]);

  const ensureMediaFile = useCallback(
    async (row: MatchRowState): Promise<string> => {
      if (row.mediaFileId) return row.mediaFileId;
      const savePath = torrent?.savePath ?? null;
      const sourceName = contextName ?? torrent?.name ?? null;

      const pathCandidates = buildPathCandidates(
        row.filePath,
        savePath,
        sourceName,
      );
      const mediaByPath = await lookupMediaByPaths(pathCandidates);
      for (const candidate of pathCandidates) {
        const media = mediaByPath.get(normalizePathForLookup(candidate));
        if (media?.id) return media.id;
      }

      const bestPath = pathCandidates[0] ?? row.filePath;
      const originalName = bestPath.split("/").pop() ?? bestPath;
      const createResult = await createUnmatchedMediaFile({
        variables: {
          input: {
            addedAt: new Date().toISOString(),
            isHdr: false,
            libraryId: UNMATCHED_LIBRARY_ID,
            metadata: JSON.stringify({
              sourceType: "torrent",
              unmatchedReason: "Created for manual media-file match dialog",
            }),
            originalName: originalName,
            path: bestPath,
            relativePath: getRelativePath(bestPath, savePath, sourceName),
            size: Math.max(0, Math.floor(row.fileSize)),
          },
        },
      });

      const mediaFileId = createResult.data?.createMediaFile?.mediaFile?.id;
      if (!createResult.data?.createMediaFile?.success || !mediaFileId) {
        throw new Error(
          createResult.data?.createMediaFile?.error ||
            "Failed to create media file",
        );
      }

      return mediaFileId;
    },
    [
      createUnmatchedMediaFile,
      lookupMediaByPaths,
      torrent?.savePath,
      torrent?.name,
      contextName,
    ],
  );

  const hydrateCandidateArtwork = useCallback(
    async (matchRows: MatchRowState[]): Promise<Record<string, string>> => {
      const movieIds = new Set<string>();
      const episodeIds = new Set<string>();
      const trackIds = new Set<string>();
      const chapterIds = new Set<string>();

      for (const row of matchRows) {
        for (const candidate of row.candidates) {
          if (candidate.targetType === "Movie")
            movieIds.add(candidate.targetId);
          if (candidate.targetType === "Episode")
            episodeIds.add(candidate.targetId);
          if (candidate.targetType === "Track")
            trackIds.add(candidate.targetId);
          if (candidate.targetType === "Chapter")
            chapterIds.add(candidate.targetId);
        }
      }

      if (
        movieIds.size === 0 &&
        episodeIds.size === 0 &&
        trackIds.size === 0 &&
        chapterIds.size === 0
      ) {
        return {};
      }

      const firstPass = await apolloClient.query<TorrentMatchCandidateArtworkRuntimeQuery>({
        query: TorrentMatchCandidateArtworkRuntimeDocument,
        variables: {
          movieIds: Array.from(movieIds),
          episodeIds: Array.from(episodeIds),
          showIds: [],
          trackIds: Array.from(trackIds),
          albumIds: [],
          chapterIds: Array.from(chapterIds),
          audiobookIds: [],
        },
        fetchPolicy: "network-only",
      });

      const episodeToShow = new Map<string, string>();
      for (const edge of firstPass.data?.episodes?.edges ?? []) {
        episodeToShow.set(edge.node.id, edge.node.showId);
      }
      const trackToAlbum = new Map<string, string>();
      for (const edge of firstPass.data?.tracks?.edges ?? []) {
        trackToAlbum.set(edge.node.id, edge.node.albumId);
      }
      const chapterToAudiobook = new Map<string, string>();
      for (const edge of firstPass.data?.chapters?.edges ?? []) {
        chapterToAudiobook.set(edge.node.id, edge.node.audiobookId);
      }

      const secondPass =
        await apolloClient.query<TorrentMatchCandidateArtworkRuntimeQuery>(
        {
          query: TorrentMatchCandidateArtworkRuntimeDocument,
          variables: {
            movieIds: [],
            episodeIds: [],
            showIds: Array.from(new Set(episodeToShow.values())),
            trackIds: [],
            albumIds: Array.from(new Set(trackToAlbum.values())),
            chapterIds: [],
            audiobookIds: Array.from(new Set(chapterToAudiobook.values())),
          },
          fetchPolicy: "network-only",
        },
      );

      const moviePoster = new Map<string, string>();
      for (const edge of firstPass.data?.movies?.edges ?? []) {
        if (edge.node.posterUrl)
          moviePoster.set(edge.node.id, edge.node.posterUrl);
      }
      const showPoster = new Map<string, string>();
      for (const edge of secondPass.data?.shows?.edges ?? []) {
        if (edge.node.posterUrl)
          showPoster.set(edge.node.id, edge.node.posterUrl);
      }
      const albumCover = new Map<string, string>();
      for (const edge of secondPass.data?.albums?.edges ?? []) {
        if (edge.node.coverUrl)
          albumCover.set(edge.node.id, edge.node.coverUrl);
      }
      const audiobookCover = new Map<string, string>();
      for (const edge of secondPass.data?.audiobooks?.edges ?? []) {
        if (edge.node.coverUrl)
          audiobookCover.set(edge.node.id, edge.node.coverUrl);
      }

      const out: Record<string, string> = {};
      for (const row of matchRows) {
        for (const candidate of row.candidates) {
          const key = `${candidate.targetType}:${candidate.targetId}`;
          if (candidate.targetType === "Movie") {
            const url = moviePoster.get(candidate.targetId);
            if (url) out[key] = url;
            continue;
          }
          if (candidate.targetType === "Episode") {
            const showId = episodeToShow.get(candidate.targetId);
            if (!showId) continue;
            const url = showPoster.get(showId);
            if (url) out[key] = url;
            continue;
          }
          if (candidate.targetType === "Track") {
            const albumId = trackToAlbum.get(candidate.targetId);
            if (!albumId) continue;
            const url = albumCover.get(albumId);
            if (url) out[key] = url;
            continue;
          }
          if (candidate.targetType === "Chapter") {
            const audiobookId = chapterToAudiobook.get(candidate.targetId);
            if (!audiobookId) continue;
            const url = audiobookCover.get(audiobookId);
            if (url) out[key] = url;
          }
        }
      }
      return out;
    },
    [],
  );

  const previewMatches = useCallback(async () => {
    if (librariesByType.length === 0) {
      addToast({
        title: "No libraries available",
        description: `No libraries found for ${formatLibraryTypeLabel(selectedLibraryType)}.`,
        color: "warning",
      });
      return;
    }

    const targetLibraryIds =
      selectedLibraryId === ALL_LIBRARIES_KEY
        ? librariesByType.map((l) => l.id)
        : [selectedLibraryId];

    setIsFinding(true);
    try {
      const nextRows = [...rows];
      for (let i = 0; i < nextRows.length; i += 1) {
        const row = nextRows[i];
        nextRows[i] = {
          ...row,
          status: "finding",
          error: null,
          candidates: [],
        };
        setRows([...nextRows]);

        try {
          const mediaFileId = await ensureMediaFile(row);
          const allCandidates: MatchCandidate[] = [];
          for (const libraryId of targetLibraryIds) {
            const result = await findMatch({
              variables: {
                input: {
                  mediaFileId,
                  libraryId,
                  autoMatch: false,
                  candidateLimit: 10,
                  force: false,
                  allowProviderFallback: false,
                },
              },
            });
            allCandidates.push(
              ...(result.data?.matchMediaFile?.candidates ?? []),
            );
          }

          const merged = mergeCandidatesByScore(allCandidates);
          const defaultKey =
            merged.length > 0
              ? `${merged[0].targetType}:${merged[0].targetId}`
              : DECLINE_KEY;

          nextRows[i] = {
            ...row,
            mediaFileId,
            candidates: merged,
            selectedKey: defaultKey,
            status: merged.length > 0 ? "ready" : "unmatched",
            error: merged.length === 0 ? "No candidates found" : null,
          };
        } catch (error) {
          nextRows[i] = {
            ...row,
            status: "error",
            error: sanitizeError(error),
          };
        }

        setRows([...nextRows]);
      }
      const artworkByKey = await hydrateCandidateArtwork(nextRows);
      setCandidateArtworkByKey(artworkByKey);
    } finally {
      setIsFinding(false);
    }
  }, [
    rows,
    librariesByType,
    selectedLibraryType,
    selectedLibraryId,
    ensureMediaFile,
    findMatch,
    hydrateCandidateArtwork,
  ]);

  useEffect(() => {
    if (!isOpen || isFinding || rows.length === 0) return;
    const scopeKey = [
      torrent?.id ?? contextName ?? "custom",
      selectedLibraryType,
      selectedLibraryId,
      rows.map((r) => r.rowId).join(","),
    ].join("|");
    if (matchScopeKeyRef.current === scopeKey) return;
    matchScopeKeyRef.current = scopeKey;
    void previewMatches();
  }, [
    isOpen,
    isFinding,
    rows,
    contextName,
    torrent?.id,
    selectedLibraryType,
    selectedLibraryId,
    previewMatches,
  ]);

  const applyMatches = useCallback(async () => {
    setIsApplying(true);
    try {
      const nextRows = [...rows];
      let appliedCount = 0;
      let unmatchedCount = 0;

      for (let i = 0; i < nextRows.length; i += 1) {
        const row = nextRows[i];
        if (!row.mediaFileId) continue;

        if (row.selectedKey === DECLINE_KEY) {
          nextRows[i] = { ...row, status: "declined", error: null };
          continue;
        }

        if (row.selectedKey === UNMATCH_KEY) {
          const result = await unmatchMediaFile({
            variables: { mediaFileId: row.mediaFileId },
          });
          if (!result.data?.unmatchMediaFile?.success) {
            nextRows[i] = {
              ...row,
              status: "error",
              error: result.data?.unmatchMediaFile?.reason || "Unmatch failed",
            };
            continue;
          }
          unmatchedCount += 1;
          nextRows[i] = {
            ...row,
            existingMatchType: null,
            existingMatchId: null,
            status: "unmatched",
            error: null,
          };
          continue;
        }

        const [targetType, targetId] = row.selectedKey.split(":");
        if (!targetType || !targetId) continue;

        const input: Record<string, unknown> = {
          mediaFileId: row.mediaFileId,
          force: true,
          autoMatch: false,
          allowProviderFallback: false,
          candidateLimit: 10,
        };
        if (targetType === "Movie") input.movieId = targetId;
        if (targetType === "Episode") input.episodeId = targetId;
        if (targetType === "Track") input.trackId = targetId;
        if (targetType === "Chapter") input.chapterId = targetId;

        const result = await findMatch({ variables: { input: input } });
        const response = result.data?.matchMediaFile;
        if (!response?.success) {
          nextRows[i] = {
            ...row,
            status: "error",
            error: response?.reason || "Match failed",
          };
          continue;
        }

        appliedCount += 1;
        nextRows[i] = {
          ...row,
          existingMatchType: targetType,
          existingMatchId: targetId,
          status: "applied",
          error: null,
        };
      }

      setRows(nextRows);
      addToast({
        title: "Match updates complete",
        description: `${appliedCount} applied, ${unmatchedCount} unmatched`,
        color: "success",
      });
      onApplied?.();
    } catch (error) {
      addToast({
        title: "Failed to apply matches",
        description: sanitizeError(error),
        color: "danger",
      });
    } finally {
      setIsApplying(false);
    }
  }, [rows, findMatch, unmatchMediaFile, onApplied]);

  const isSingleFileMode = initialFileIndex != null;
  const isLoading = torrentLoading || librariesLoading;
  const matchedCount = rows.filter(
    (r) =>
      r.status !== "error" &&
      r.status !== "finding" &&
      r.status !== "idle" &&
      r.selectedKey !== DECLINE_KEY &&
      r.selectedKey !== UNMATCH_KEY,
  ).length;
  const totalRows = rows.length;
  const progressCount = rows.filter(
    (r) => r.status !== "idle" && r.status !== "finding",
  ).length;

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="4xl" scrollBehavior="inside">
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1">
          <span>{isSingleFileMode ? "Match File" : "Match Media Files"}</span>
          {(contextName ?? torrent?.name) && (
            <span className="text-xs text-default-500 font-normal truncate">
              {contextName ?? torrent?.name}
            </span>
          )}
        </ModalHeader>

        <ModalBody className="space-y-4">
          {isLoading ? (
            <div className="py-12 flex items-center justify-center">
              <Spinner size="lg" />
            </div>
          ) : (
            <>
              {/* Library filter controls */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                <Select
                  label="Library Type"
                  size="sm"
                  selectedKeys={[selectedLibraryType]}
                  onSelectionChange={(keys) => {
                    const key = Array.from(keys)[0]?.toString();
                    if (key) setSelectedLibraryType(key);
                  }}
                  classNames={{
                    trigger: "bg-content3 border border-default-200",
                  }}
                >
                  {[
                    <SelectItem
                      key={ALL_LIBRARY_TYPES_KEY}
                      textValue={formatLibraryTypeLabel(ALL_LIBRARY_TYPES_KEY)}
                    >
                      {formatLibraryTypeLabel(ALL_LIBRARY_TYPES_KEY)}
                    </SelectItem>,
                    ...availableLibraryTypes.map((type) => (
                      <SelectItem
                        key={type}
                        textValue={formatLibraryTypeLabel(type)}
                      >
                        {formatLibraryTypeLabel(type)}
                      </SelectItem>
                    )),
                  ]}
                </Select>
                <Select
                  label="Library Scope"
                  size="sm"
                  selectedKeys={[selectedLibraryId]}
                  onSelectionChange={(keys) => {
                    const key = Array.from(keys)[0]?.toString();
                    if (key) setSelectedLibraryId(key);
                  }}
                  classNames={{
                    trigger: "bg-content3 border border-default-200",
                  }}
                >
                  {[
                    <SelectItem
                      key={ALL_LIBRARIES_KEY}
                      textValue="All libraries"
                    >
                      All libraries
                    </SelectItem>,
                    ...librariesByType.map((library) => (
                      <SelectItem key={library.id} textValue={library.name}>
                        {library.name}
                      </SelectItem>
                    )),
                  ]}
                </Select>
              </div>

              {/* Progress bar when matching */}
              {isFinding && totalRows > 1 && (
                <Progress
                  size="sm"
                  value={(progressCount / totalRows) * 100}
                  color="primary"
                  label={`Matching ${progressCount}/${totalRows} files...`}
                  showValueLabel
                  classNames={{ label: "text-xs text-default-500" }}
                />
              )}

              {/* Status bar */}
              <div className="flex items-center justify-between gap-2">
                <span className="text-xs text-default-500">
                  Matches refresh automatically when scope changes
                </span>
                {matchedCount > 0 && (
                  <span className="text-xs text-default-500">
                    {matchedCount} of {totalRows} matched
                  </span>
                )}
              </div>

              <Divider />

              {/* File match rows — split view */}
              <div className="space-y-3">
                {rows.map((row) => {
                  const selectedCandidate =
                    row.candidates.find(
                      (c) =>
                        `${c.targetType}:${c.targetId}` === row.selectedKey,
                    ) ?? null;

                  return (
                    <div
                      key={row.rowId}
                      className="grid grid-cols-1 md:grid-cols-2 gap-3"
                    >
                      {/* Left: File info */}
                      <FileInfoPanel row={row} />

                      {/* Right: Match result card */}
                      <MatchCandidateCard
                        candidate={selectedCandidate}
                        allCandidates={row.candidates}
                        artworkUrl={
                          selectedCandidate
                            ? candidateArtworkByKey[
                                `${selectedCandidate.targetType}:${selectedCandidate.targetId}`
                              ]
                            : undefined
                        }
                        selectedKey={row.selectedKey}
                        onSelect={(key) =>
                          setRows((prev) =>
                            prev.map((r) =>
                              r.rowId === row.rowId
                                ? { ...r, selectedKey: key }
                                : r,
                            ),
                          )
                        }
                        status={row.status}
                        error={row.error}
                      />
                    </div>
                  );
                })}

                {rows.length === 0 && (
                  <div className="text-sm text-default-500 py-8 text-center border border-dashed border-default-300 rounded-lg bg-content1/50">
                    No files available for matching.
                  </div>
                )}
              </div>
            </>
          )}
        </ModalBody>

        <ModalFooter className="flex justify-between">
          <Button
            variant="flat"
            size="sm"
            startContent={<IconX size={16} />}
            onPress={onClose}
          >
            Close
          </Button>
          <Button
            color="success"
            size="sm"
            startContent={<IconCheck size={16} />}
            onPress={() => void applyMatches()}
            isLoading={isApplying}
            isDisabled={rows.length === 0 || matchedCount === 0}
          >
            Apply {matchedCount > 0 ? `(${matchedCount})` : "Selected"}
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
