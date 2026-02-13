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
  type CreateUnmatchedMediaFileFromTorrentMutation,
  type CreateUnmatchedMediaFileFromTorrentMutationVariables,
  type TorrentByInfoHashWithFilesQuery,
  type TorrentByInfoHashWithFilesQueryVariables,
} from "../../lib/graphql/generated/graphql";
import {
  apolloClient,
  gql,
  useMutation,
  useQuery,
} from "../../lib/graphql/client";
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
// GraphQL Operations
// ============================================================================

const TORRENT_MATCH_LIBRARIES_QUERY = gql(`
  query TorrentMatchLibrariesRuntime {
    Libraries(Page: { Limit: 1000, Offset: 0 }) {
      Edges {
        Node {
          Id
          Name
          LibraryType
        }
      }
    }
  }
`);

const TORRENT_MEDIA_LOOKUP_QUERY = gql(`
  query TorrentMatchMediaFilesByPathsRuntime($Paths: [String!]!) {
    MediaFiles(Where: { Path: { In: $Paths } }, Page: { Limit: 1000, Offset: 0 }) {
      Edges {
        Node {
          Id
          Path
          Metadata
          LibraryId
          EpisodeId
          MovieId
          TrackId
          ChapterId
        }
      }
    }
  }
`);

const TORRENT_FIND_MATCH_MUTATION = gql(`
  mutation TorrentFindMatchForMediaFileRuntime($Input: MatchMediaFileInput!) {
    MatchMediaFile(Input: $Input) {
      Success
      AutoMatched
      AlreadyMatched
      MatchedType
      MatchedId
      Confidence
      Reason
      Candidates {
        TargetType
        TargetId
        TargetName
        Score
        Reason
        Wanted
      }
    }
  }
`);

const TORRENT_UNMATCH_MEDIA_FILE_MUTATION = gql(`
  mutation TorrentUnmatchMediaFileRuntime($MediaFileId: String!) {
    UnmatchMediaFile(MediaFileId: $MediaFileId) {
      Success
      Reason
    }
  }
`);

const TORRENT_MATCH_ARTWORK_QUERY = gql(`
  query TorrentMatchCandidateArtworkRuntime(
    $MovieIds: [String!]!
    $EpisodeIds: [String!]!
    $ShowIds: [String!]!
    $TrackIds: [String!]!
    $AlbumIds: [String!]!
    $ChapterIds: [String!]!
    $AudiobookIds: [String!]!
  ) {
    Movies(Where: { Id: { In: $MovieIds } }, Page: { Limit: 1000, Offset: 0 }) {
      Edges {
        Node {
          Id
          PosterUrl
        }
      }
    }
    Episodes(Where: { Id: { In: $EpisodeIds } }, Page: { Limit: 1000, Offset: 0 }) {
      Edges {
        Node {
          Id
          ShowId
        }
      }
    }
    Shows(Where: { Id: { In: $ShowIds } }, Page: { Limit: 1000, Offset: 0 }) {
      Edges {
        Node {
          Id
          PosterUrl
        }
      }
    }
    Tracks(Where: { Id: { In: $TrackIds } }, Page: { Limit: 1000, Offset: 0 }) {
      Edges {
        Node {
          Id
          AlbumId
        }
      }
    }
    Albums(Where: { Id: { In: $AlbumIds } }, Page: { Limit: 1000, Offset: 0 }) {
      Edges {
        Node {
          Id
          CoverUrl
        }
      }
    }
    Chapters(Where: { Id: { In: $ChapterIds } }, Page: { Limit: 1000, Offset: 0 }) {
      Edges {
        Node {
          Id
          AudiobookId
        }
      }
    }
    Audiobooks(
      Where: { Id: { In: $AudiobookIds } }
      Page: { Limit: 1000, Offset: 0 }
    ) {
      Edges {
        Node {
          Id
          CoverUrl
        }
      }
    }
  }
`);

// ============================================================================
// Types
// ============================================================================

type LibraryNode = {
  Id: string;
  Name: string;
  LibraryType: string;
};

type TorrentFileNode =
  TorrentByInfoHashWithFilesQuery["Torrents"]["Edges"][number]["Node"]["Files"]["Edges"][number]["Node"];

type MediaLookupNode = {
  Id: string;
  Path: string;
  Metadata?: string | null;
  LibraryId?: string | null;
  EpisodeId?: string | null;
  MovieId?: string | null;
  TrackId?: string | null;
  ChapterId?: string | null;
};

type MatchCandidate = {
  TargetType: string;
  TargetId: string;
  TargetName?: string | null;
  Score: number;
  Reason?: string | null;
  Wanted?: boolean | null;
};

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

interface MatchMediaFileResult {
  Success: boolean;
  Reason?: string | null;
  Candidates?: MatchCandidate[];
}

interface TorrentMatchArtworkQueryData {
  Movies?: {
    Edges: Array<{ Node: { Id: string; PosterUrl?: string | null } }>;
  };
  Episodes?: {
    Edges: Array<{ Node: { Id: string; ShowId: string } }>;
  };
  Shows?: {
    Edges: Array<{ Node: { Id: string; PosterUrl?: string | null } }>;
  };
  Tracks?: {
    Edges: Array<{ Node: { Id: string; AlbumId: string } }>;
  };
  Albums?: {
    Edges: Array<{ Node: { Id: string; CoverUrl?: string | null } }>;
  };
  Chapters?: {
    Edges: Array<{ Node: { Id: string; AudiobookId: string } }>;
  };
  Audiobooks?: {
    Edges: Array<{ Node: { Id: string; CoverUrl?: string | null } }>;
  };
}

interface TorrentFindMatchMutationData {
  MatchMediaFile: MatchMediaFileResult;
}

interface TorrentUnmatchMediaFileMutationData {
  UnmatchMediaFile: { Success: boolean; Reason?: string | null };
}

interface TorrentMatchLibrariesQueryData {
  Libraries: {
    Edges: Array<{ Node: LibraryNode }>;
  };
}

interface TorrentMediaLookupQueryData {
  MediaFiles: {
    Edges: Array<{ Node: MediaLookupNode }>;
  };
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
  FileIndex?: number | null;
  FilePath: string;
  FileSize: number;
  MediaFileId?: string | null;
  EpisodeId?: string | null;
  MovieId?: string | null;
  TrackId?: string | null;
  ChapterId?: string | null;
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
  torrentName?: string | null
): string[] {
  const candidates = new Set<string>();
  const normalizedFilePath = filePath.replace(/\\/g, "/");
  candidates.add(filePath);
  candidates.add(normalizedFilePath);

  if (savePath && !isAbsolutePath(normalizedFilePath)) {
    candidates.add(joinPath(savePath, normalizedFilePath));
    if (torrentName) {
      candidates.add(
        joinPath(joinPath(savePath, torrentName), normalizedFilePath)
      );
    }
  }

  return Array.from(candidates);
}

function getRelativePath(
  filePath: string,
  savePath?: string | null,
  torrentName?: string | null
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
    const key = `${candidate.TargetType}:${candidate.TargetId}`;
    const current = merged.get(key);
    if (!current || candidate.Score > current.Score) {
      merged.set(key, candidate);
    }
  }
  return Array.from(merged.values())
    .sort((a, b) => b.Score - a.Score)
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
  status: MatchRowState["status"]
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
            <IconArrowsShuffle
              size={28}
              className="text-default-400 mx-auto"
            />
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
            <IconAlertTriangle
              size={28}
              className="text-danger-400 mx-auto"
            />
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
                  <IconUnlink
                    size={28}
                    className="text-warning-400 mx-auto"
                  />
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
                  const key = `${c.TargetType}:${c.TargetId}`;
                  const label =
                    c.TargetName ?? `${c.TargetType} ${c.TargetId.slice(0, 8)}`;
                  const textVal = `${label} (${(c.Score * 100).toFixed(0)}%)`;
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
  const gradient = getTypeGradient(candidate.TargetType);
  const displayName =
    candidate.TargetName ??
    `${candidate.TargetType} ${candidate.TargetId.slice(0, 8)}...`;
  const confidence = Math.round(candidate.Score * 100);
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
            getTypeIcon(candidate.TargetType, 28)
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
              {candidate.Wanted && (
                <Chip size="sm" variant="flat" color="primary" className="h-5">
                  Wanted
                </Chip>
              )}
              <span className="text-xs text-white/60">
                {candidate.TargetType}
              </span>
            </div>
          </div>
          <Checkbox
            isSelected={isSelected}
            onValueChange={(value) => {
              if (value) {
                onSelect(`${candidate.TargetType}:${candidate.TargetId}`);
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
                  const key = `${c.TargetType}:${c.TargetId}`;
                  const label =
                    c.TargetName ??
                    `${c.TargetType} ${c.TargetId.slice(0, 8)}`;
                  const textVal = `${label} (${(c.Score * 100).toFixed(0)}%)`;
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
function FileInfoPanel({
  row,
}: {
  row: MatchRowState;
}) {
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
          <Chip
            size="sm"
            variant="flat"
            color={getStatusColor(row.status)}
          >
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
  const [selectedLibraryType, setSelectedLibraryType] =
    useState<string>(ALL_LIBRARY_TYPES_KEY);
  const [selectedLibraryId, setSelectedLibraryId] =
    useState<string>(ALL_LIBRARIES_KEY);
  const matchScopeKeyRef = useRef<string | null>(null);

  const torrentQueryVariables =
    useMemo<TorrentByInfoHashWithFilesQueryVariables>(
      () => ({
        Where: { InfoHash: { Eq: torrentInfoHash ?? "" } },
        Page: { Limit: 1, Offset: 0 },
      }),
      [torrentInfoHash]
    );

  const { data: torrentData, loading: torrentLoading } = useQuery(
    TorrentByInfoHashWithFilesDocument,
    {
      variables: torrentQueryVariables,
      skip: !isOpen || !torrentInfoHash || (mediaFiles?.length ?? 0) > 0,
      fetchPolicy: "cache-and-network",
      notifyOnNetworkStatusChange: true,
    }
  );
  const { data: librariesData, loading: librariesLoading } = useQuery<
    TorrentMatchLibrariesQueryData
  >(TORRENT_MATCH_LIBRARIES_QUERY, {
    skip: !isOpen,
    fetchPolicy: "cache-and-network",
    notifyOnNetworkStatusChange: true,
  });

  const [createUnmatchedMediaFile] = useMutation<
    CreateUnmatchedMediaFileFromTorrentMutation,
    CreateUnmatchedMediaFileFromTorrentMutationVariables
  >(CreateUnmatchedMediaFileFromTorrentDocument);
  const [findMatch] = useMutation<TorrentFindMatchMutationData>(
    TORRENT_FIND_MATCH_MUTATION
  );
  const [unmatchMediaFile] = useMutation<TorrentUnmatchMediaFileMutationData>(
    TORRENT_UNMATCH_MEDIA_FILE_MUTATION
  );

  const torrent = torrentData?.Torrents?.Edges?.[0]?.Node ?? null;
  const torrentFiles = useMemo<TorrentFileNode[]>(() => {
    const files =
      torrent?.Files?.Edges
        ?.map((e) => e.Node)
        .filter((f) => isMatchableMediaFile(f.FilePath)) ?? [];
    if (initialFileIndex == null) return files;
    return files.filter((f) => f.FileIndex === initialFileIndex);
  }, [torrent?.Files?.Edges, initialFileIndex]);
  const inputMediaFiles = useMemo<MediaFileMatchInput[]>(() => {
    if (!mediaFiles || mediaFiles.length === 0) return [];
    const files = mediaFiles.filter((f) => isMatchableMediaFile(f.FilePath));
    if (initialFileIndex == null) return files;
    return files.filter((f) => f.FileIndex === initialFileIndex);
  }, [mediaFiles, initialFileIndex]);
  const sourceFiles = useMemo<MediaFileMatchInput[]>(() => {
    if (inputMediaFiles.length > 0) return inputMediaFiles;
    return torrentFiles.map((f) => ({
      RowId: `torrent:${f.FileIndex}`,
      FileIndex: f.FileIndex,
      FilePath: f.FilePath,
      FileSize: f.FileSize,
    }));
  }, [inputMediaFiles, torrentFiles]);

  const allLibraries = useMemo<LibraryNode[]>(
    () => librariesData?.Libraries?.Edges?.map((e) => e.Node) ?? [],
    [librariesData?.Libraries?.Edges]
  );

  const availableLibraryTypes = useMemo(
    () =>
      Array.from(
        new Set(
          allLibraries
            .map((l) => normalizeLibraryType(l.LibraryType))
            .filter(Boolean)
        )
      ).sort(),
    [allLibraries]
  );

  const librariesByType = useMemo(
    () => {
      if (selectedLibraryType === ALL_LIBRARY_TYPES_KEY) {
        return allLibraries;
      }
      return allLibraries.filter(
        (l) => normalizeLibraryType(l.LibraryType) === selectedLibraryType
      );
    },
    [allLibraries, selectedLibraryType]
  );

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
    const result = await apolloClient.query<TorrentMediaLookupQueryData>({
      query: TORRENT_MEDIA_LOOKUP_QUERY,
      variables: { Paths: paths },
      fetchPolicy: "network-only",
    });
    const map = new Map<string, MediaLookupNode>();
    const edges = result.data?.MediaFiles?.Edges ?? [];
    for (const edge of edges) {
      map.set(normalizePathForLookup(edge.Node.Path), edge.Node);
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
      const savePath = torrent?.SavePath ?? null;
      const sourceName = contextName ?? torrent?.Name ?? null;
      const allPathCandidates = Array.from(
        new Set(
          sourceFiles.flatMap((f) =>
            buildPathCandidates(f.FilePath, savePath, sourceName)
          )
        )
      );

      const mediaByPath = await lookupMediaByPaths(allPathCandidates);

      const nextRows = sourceFiles.map((f, idx) => {
        const rowId = f.RowId ?? `row:${f.FileIndex ?? idx}`;
        const candidates = buildPathCandidates(
          f.FilePath,
          savePath,
          sourceName
        );
        let media: MediaLookupNode | null =
          f.MediaFileId != null
            ? ({
                Id: f.MediaFileId,
                Path: f.FilePath,
                EpisodeId: f.EpisodeId ?? null,
                MovieId: f.MovieId ?? null,
                TrackId: f.TrackId ?? null,
                ChapterId: f.ChapterId ?? null,
              } as MediaLookupNode)
            : null;
        if (!media) {
          for (const candidate of candidates) {
            media = mediaByPath.get(normalizePathForLookup(candidate)) ?? null;
            if (media) break;
          }
        }
        const existingMatchType = media?.EpisodeId
          ? "Episode"
          : media?.MovieId
            ? "Movie"
            : media?.TrackId
              ? "Track"
              : media?.ChapterId
                ? "Chapter"
                : null;
        const existingMatchId =
          media?.EpisodeId ??
          media?.MovieId ??
          media?.TrackId ??
          media?.ChapterId ??
          null;

        return {
          rowId,
          fileIndex: f.FileIndex ?? null,
          filePath: f.FilePath,
          fileSize: f.FileSize,
          mediaFileId: f.MediaFileId ?? media?.Id ?? null,
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
  }, [isOpen, sourceFiles, lookupMediaByPaths, torrent?.SavePath, torrent?.Name, contextName]);

  useEffect(() => {
    void hydrateRows();
  }, [hydrateRows]);

  const ensureMediaFile = useCallback(
    async (row: MatchRowState): Promise<string> => {
      if (row.mediaFileId) return row.mediaFileId;
      const savePath = torrent?.SavePath ?? null;
      const sourceName = contextName ?? torrent?.Name ?? null;

      const pathCandidates = buildPathCandidates(
        row.filePath,
        savePath,
        sourceName
      );
      const mediaByPath = await lookupMediaByPaths(pathCandidates);
      for (const candidate of pathCandidates) {
        const media = mediaByPath.get(normalizePathForLookup(candidate));
        if (media?.Id) return media.Id;
      }

      const bestPath = pathCandidates[0] ?? row.filePath;
      const originalName = bestPath.split("/").pop() ?? bestPath;
      const createResult = await createUnmatchedMediaFile({
        variables: {
          Input: {
            AddedAt: new Date().toISOString(),
            IsHdr: false,
            LibraryId: UNMATCHED_LIBRARY_ID,
            Metadata: JSON.stringify({
              SourceType: "torrent",
              UnmatchedReason:
                "Created for manual media-file match dialog",
            }),
            OriginalName: originalName,
            Path: bestPath,
            RelativePath: getRelativePath(
              bestPath,
              savePath,
              sourceName
            ),
            Size: Math.max(0, Math.floor(row.fileSize)),
          },
        },
      });

      const mediaFileId =
        createResult.data?.CreateMediaFile?.MediaFile?.Id;
      if (!createResult.data?.CreateMediaFile?.Success || !mediaFileId) {
        throw new Error(
          createResult.data?.CreateMediaFile?.Error ||
            "Failed to create media file"
        );
      }

      return mediaFileId;
    },
    [createUnmatchedMediaFile, lookupMediaByPaths, torrent?.SavePath, torrent?.Name, contextName]
  );

  const hydrateCandidateArtwork = useCallback(
    async (matchRows: MatchRowState[]): Promise<Record<string, string>> => {
      const movieIds = new Set<string>();
      const episodeIds = new Set<string>();
      const trackIds = new Set<string>();
      const chapterIds = new Set<string>();

      for (const row of matchRows) {
        for (const candidate of row.candidates) {
          if (candidate.TargetType === "Movie") movieIds.add(candidate.TargetId);
          if (candidate.TargetType === "Episode") episodeIds.add(candidate.TargetId);
          if (candidate.TargetType === "Track") trackIds.add(candidate.TargetId);
          if (candidate.TargetType === "Chapter") chapterIds.add(candidate.TargetId);
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

      const firstPass = await apolloClient.query<TorrentMatchArtworkQueryData>({
        query: TORRENT_MATCH_ARTWORK_QUERY,
        variables: {
          MovieIds: Array.from(movieIds),
          EpisodeIds: Array.from(episodeIds),
          ShowIds: [],
          TrackIds: Array.from(trackIds),
          AlbumIds: [],
          ChapterIds: Array.from(chapterIds),
          AudiobookIds: [],
        },
        fetchPolicy: "network-only",
      });

      const episodeToShow = new Map<string, string>();
      for (const edge of firstPass.data?.Episodes?.Edges ?? []) {
        episodeToShow.set(edge.Node.Id, edge.Node.ShowId);
      }
      const trackToAlbum = new Map<string, string>();
      for (const edge of firstPass.data?.Tracks?.Edges ?? []) {
        trackToAlbum.set(edge.Node.Id, edge.Node.AlbumId);
      }
      const chapterToAudiobook = new Map<string, string>();
      for (const edge of firstPass.data?.Chapters?.Edges ?? []) {
        chapterToAudiobook.set(edge.Node.Id, edge.Node.AudiobookId);
      }

      const secondPass = await apolloClient.query<TorrentMatchArtworkQueryData>({
        query: TORRENT_MATCH_ARTWORK_QUERY,
        variables: {
          MovieIds: [],
          EpisodeIds: [],
          ShowIds: Array.from(new Set(episodeToShow.values())),
          TrackIds: [],
          AlbumIds: Array.from(new Set(trackToAlbum.values())),
          ChapterIds: [],
          AudiobookIds: Array.from(new Set(chapterToAudiobook.values())),
        },
        fetchPolicy: "network-only",
      });

      const moviePoster = new Map<string, string>();
      for (const edge of firstPass.data?.Movies?.Edges ?? []) {
        if (edge.Node.PosterUrl) moviePoster.set(edge.Node.Id, edge.Node.PosterUrl);
      }
      const showPoster = new Map<string, string>();
      for (const edge of secondPass.data?.Shows?.Edges ?? []) {
        if (edge.Node.PosterUrl) showPoster.set(edge.Node.Id, edge.Node.PosterUrl);
      }
      const albumCover = new Map<string, string>();
      for (const edge of secondPass.data?.Albums?.Edges ?? []) {
        if (edge.Node.CoverUrl) albumCover.set(edge.Node.Id, edge.Node.CoverUrl);
      }
      const audiobookCover = new Map<string, string>();
      for (const edge of secondPass.data?.Audiobooks?.Edges ?? []) {
        if (edge.Node.CoverUrl) audiobookCover.set(edge.Node.Id, edge.Node.CoverUrl);
      }

      const out: Record<string, string> = {};
      for (const row of matchRows) {
        for (const candidate of row.candidates) {
          const key = `${candidate.TargetType}:${candidate.TargetId}`;
          if (candidate.TargetType === "Movie") {
            const url = moviePoster.get(candidate.TargetId);
            if (url) out[key] = url;
            continue;
          }
          if (candidate.TargetType === "Episode") {
            const showId = episodeToShow.get(candidate.TargetId);
            if (!showId) continue;
            const url = showPoster.get(showId);
            if (url) out[key] = url;
            continue;
          }
          if (candidate.TargetType === "Track") {
            const albumId = trackToAlbum.get(candidate.TargetId);
            if (!albumId) continue;
            const url = albumCover.get(albumId);
            if (url) out[key] = url;
            continue;
          }
          if (candidate.TargetType === "Chapter") {
            const audiobookId = chapterToAudiobook.get(candidate.TargetId);
            if (!audiobookId) continue;
            const url = audiobookCover.get(audiobookId);
            if (url) out[key] = url;
          }
        }
      }
      return out;
    },
    []
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
        ? librariesByType.map((l) => l.Id)
        : [selectedLibraryId];

    setIsFinding(true);
    try {
      const nextRows = [...rows];
      for (let i = 0; i < nextRows.length; i += 1) {
        const row = nextRows[i];
        nextRows[i] = { ...row, status: "finding", error: null, candidates: [] };
        setRows([...nextRows]);

        try {
          const mediaFileId = await ensureMediaFile(row);
          const allCandidates: MatchCandidate[] = [];
          for (const libraryId of targetLibraryIds) {
            const result = await findMatch({
              variables: {
                Input: {
                  MediaFileId: mediaFileId,
                  LibraryId: libraryId,
                  AutoMatch: false,
                  CandidateLimit: 10,
                  Force: false,
                  AllowProviderFallback: false,
                },
              },
            });
            allCandidates.push(
              ...(result.data?.MatchMediaFile?.Candidates ?? [])
            );
          }

          const merged = mergeCandidatesByScore(allCandidates);
          const defaultKey =
            merged.length > 0
              ? `${merged[0].TargetType}:${merged[0].TargetId}`
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
      torrent?.Id ?? contextName ?? "custom",
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
    torrent?.Id,
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
            variables: { MediaFileId: row.mediaFileId },
          });
          if (!result.data?.UnmatchMediaFile?.Success) {
            nextRows[i] = {
              ...row,
              status: "error",
              error:
                result.data?.UnmatchMediaFile?.Reason || "Unmatch failed",
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
          MediaFileId: row.mediaFileId,
          Force: true,
          AutoMatch: false,
          AllowProviderFallback: false,
          CandidateLimit: 10,
        };
        if (targetType === "Movie") input.MovieId = targetId;
        if (targetType === "Episode") input.EpisodeId = targetId;
        if (targetType === "Track") input.TrackId = targetId;
        if (targetType === "Chapter") input.ChapterId = targetId;

        const result = await findMatch({ variables: { Input: input } });
        const response = result.data?.MatchMediaFile;
        if (!response?.Success) {
          nextRows[i] = {
            ...row,
            status: "error",
            error: response?.Reason || "Match failed",
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
      r.selectedKey !== UNMATCH_KEY
  ).length;
  const totalRows = rows.length;
  const progressCount = rows.filter(
    (r) => r.status !== "idle" && r.status !== "finding"
  ).length;

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="4xl"
      scrollBehavior="inside"
    >
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1">
          <span>
            {isSingleFileMode ? "Match File" : "Match Media Files"}
          </span>
          {(contextName ?? torrent?.Name) && (
            <span className="text-xs text-default-500 font-normal truncate">
              {contextName ?? torrent?.Name}
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
                  classNames={{ trigger: "bg-content3 border border-default-200" }}
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
                  classNames={{ trigger: "bg-content3 border border-default-200" }}
                >
                  {[
                    <SelectItem key={ALL_LIBRARIES_KEY} textValue="All libraries">
                      All libraries
                    </SelectItem>,
                    ...librariesByType.map((library) => (
                      <SelectItem key={library.Id} textValue={library.Name}>
                        {library.Name}
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
                        `${c.TargetType}:${c.TargetId}` === row.selectedKey
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
                                `${selectedCandidate.TargetType}:${selectedCandidate.TargetId}`
                              ]
                            : undefined
                        }
                        selectedKey={row.selectedKey}
                        onSelect={(key) =>
                          setRows((prev) =>
                            prev.map((r) =>
                              r.rowId === row.rowId
                                ? { ...r, selectedKey: key }
                                : r
                            )
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
