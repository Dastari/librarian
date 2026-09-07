import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Input } from "@heroui/input";
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from "@heroui/modal";
import { Tooltip } from "@heroui/tooltip";
import { addToast } from "@heroui/toast";
import { IconDownload, IconSearch } from "@tabler/icons-react";
import { DataTable } from "./data-table/DataTable";
import type { DataTableColumn } from "./data-table/types";
import { apolloClient, useMutation } from "../lib/graphql/client";
import { SearchSourcesDocument, AddTorrentDocument, type SearchSourcesQuery, type SearchSourcesInput } from "../lib/graphql/generated/graphql";
import { sanitizeError } from "../lib/format";

type SourceReleaseInfo = SearchSourcesQuery["searchSources"]["sources"][number]["releases"][number];

function initialSearchText(input?: SearchSourcesInput): string {
  const query = input?.query.trim() ?? "";
  if (input?.season == null) return query;
  const season = `S${String(input.season).padStart(2, "0")}`;
  const episode = input.episode == null ? "" : `E${input.episode.padStart(2, "0")}`;
  return `${query} ${season}${episode}`.trim();
}

function searchInputFromText(text: string, defaults?: SearchSourcesInput): SearchSourcesInput {
  // Derive episode criteria from the visible text so changing or removing the
  // episode never leaves the original episode attached as a hidden restriction.
  const query = text.trim();
  const token = /(?:^|\s)S(\d+)(?:E(\d+))?$/i.exec(query);
  return {
    ...defaults,
    query: token ? query.slice(0, token.index).trim() : query,
    season: token ? Number(token[1]) : undefined,
    episode: token?.[2] == null ? undefined : String(Number(token[2])),
  };
}

export interface SearchSourcesModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialSearch?: SearchSourcesInput;
  title?: string;
  libraryId?: string;
  showId?: string;
  movieId?: string;
}

export function SearchSourcesModal({
  isOpen, onClose, initialSearch, title = "Search All Sources", libraryId, showId, movieId,
}: SearchSourcesModalProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SourceReleaseInfo[]>([]);
  const [searching, setSearching] = useState(false);
  const [searchErrors, setSearchErrors] = useState<string[]>([]);
  const [hasSearched, setHasSearched] = useState(false);
  const requestId = useRef(0);

  const runSearch = useCallback(async (input: SearchSourcesInput) => {
    if (!input.query.trim()) return;
    const currentRequest = ++requestId.current;
    setSearching(true);
    setHasSearched(true);
    setSearchResults([]);
    setSearchErrors([]);
    try {
      const { data } = await apolloClient.query({
        query: SearchSourcesDocument, variables: { input }, fetchPolicy: "network-only",
      });
      if (requestId.current !== currentRequest) return;
      if (!data?.searchSources) throw new Error("Search returned no response. Please try again.");
      const result = data.searchSources;
      setSearchResults(result.sources.flatMap(source => source.releases));
      setSearchErrors(result.sourcesSearched === 0
        ? ["No enabled sources are available. Configure a source before searching for releases."]
        : result.sources.filter(source => source.error).map(source => `${source.sourceName}: ${sanitizeError(source.error)}`));
    } catch (error) {
      if (requestId.current === currentRequest) setSearchErrors([sanitizeError(error)]);
    } finally {
      if (requestId.current === currentRequest) setSearching(false);
    }
  }, []);

  useEffect(() => {
    const text = initialSearchText(initialSearch);
    setSearchQuery(text);
    setSearchResults([]);
    setSearchErrors([]);
    setHasSearched(false);
    setSearching(false);
    if (isOpen && text) void runSearch(searchInputFromText(text, initialSearch));
    return () => { requestId.current++; };
  }, [isOpen, initialSearch, runSearch]);

  const onSearch = () => void runSearch(searchInputFromText(searchQuery, initialSearch));
  const [addTorrent] = useMutation(AddTorrentDocument);
  const [addingReleaseKey, setAddingReleaseKey] = useState<string | null>(null);

  const getReleaseKey = useCallback(
    (release: SourceReleaseInfo) =>
      `${release.guid}:${release.sourceId ?? release.sourceName ?? ""}:${release.title}`,
    [],
  );

  const handleAddToDownloads = useCallback(
    async (release: SourceReleaseInfo) => {
      const magnetUri = release.magnetUri ?? undefined;
      const torrentUrl = release.link ?? undefined;

      if (!magnetUri && !torrentUrl) {
        addToast({
          title: "No Download Link",
          description: "This release does not include a magnet or torrent URL.",
          color: "warning",
        });
        return;
      }

      const isMagnet = magnetUri?.startsWith("magnet:");
      const releaseKey = getReleaseKey(release);
      setAddingReleaseKey(releaseKey);

      try {
        const result = await addTorrent({
          variables: {
            input: {
              libraryId,
              showId,
              movieId,
              magnet: isMagnet ? magnetUri : undefined,
              url: !isMagnet ? magnetUri || torrentUrl : undefined,
              sourceUrl: torrentUrl || magnetUri,
              sourceIndexerId: release.sourceId || release.sourceName || undefined,
            },
          },
        });

        const data = result.data?.addTorrent;
        if (data?.success && data.torrent) {
          addToast({
            title: "Torrent Added",
            description: `Started downloading: ${data.torrent.name}`,
            color: "success",
          });
          return;
        }

        addToast({
          title: "Failed to Add Torrent",
          description: sanitizeError(
            data?.error ?? result.error?.message ?? "Unknown error",
          ),
          color: "danger",
        });
      } catch (error) {
        addToast({
          title: "Failed to Add Torrent",
          description: sanitizeError(error),
          color: "danger",
        });
      } finally {
        setAddingReleaseKey(null);
      }
    },
    [addTorrent, getReleaseKey, libraryId, showId, movieId],
  );

  const columns = useMemo<DataTableColumn<SourceReleaseInfo>[]>(
    () => [
      {
        key: "title",
        label: "Title",
        sortable: true,
        render: (release) => (
          <div className="truncate" title={release.title}>
            {release.details ? (
              <a
                href={release.details}
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline"
              >
                {release.title}
              </a>
            ) : (
              release.title
            )}
          </div>
        ),
        width: 500,
      },
      {
        key: "size",
        label: "Size",
        sortable: true,
        align: "end",
        render: (release) => (
          <span className="text-default-500 tabular-nums whitespace-nowrap">
            {release.sizeFormatted ?? "-"}
          </span>
        ),
      },
      {
        key: "seeders",
        label: "Seeds",
        sortable: true,
        align: "end",
        render: (release) => (
          <span className="text-green-400 tabular-nums">
            {release.seeders ?? "-"}
          </span>
        ),
      },
      {
        key: "leechers",
        label: "Leech",
        sortable: true,
        align: "end",
        render: (release) => (
          <span className="text-red-400 tabular-nums">
            {release.leechers ?? "-"}
          </span>
        ),
      },
      {
        key: "sourceName",
        label: "Source",
        sortable: true,
        render: (release) => (
          <span className="text-default-500 text-xs">
            {release.sourceName ?? "-"}
          </span>
        ),
      },
      {
        key: "isFreeleech",
        label: "FL",
        sortable: true,
        align: "center",
        render: (release) =>
          release.isFreeleech ? (
            <Chip size="sm" variant="flat" color="success">
              FL
            </Chip>
          ) : (
            "-"
          ),
      },
      {
        key: "actions",
        label: "Actions",
        sortable: false,
        align: "end",
        render: (release) => {
          const canDownload = Boolean(release.link ?? release.magnetUri);
          const tooltipText = canDownload ? "Add to downloads" : null;
          const releaseKey = getReleaseKey(release);

          return canDownload && tooltipText ? (
            <Tooltip content={tooltipText}>
              <Button
                isIconOnly
                aria-label={`Download ${release.title}`}
                size="sm"
                variant="light"
                isLoading={addingReleaseKey === releaseKey}
                onPress={() => void handleAddToDownloads(release)}
              >
                <IconDownload size={14} className="text-blue-400" />
              </Button>
            </Tooltip>
          ) : (
            <span className="text-default-400">-</span>
          );
        },
      },
    ],
    [addingReleaseKey, getReleaseKey, handleAddToDownloads],
  );

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="5xl"
      scrollBehavior="inside"
      classNames={{ base: "h-[85dvh]", body: "min-h-0" }}
    >
      <ModalContent>
        <ModalHeader>{title}</ModalHeader>
        <ModalBody>
          <div className="flex h-0 min-h-0 grow flex-col gap-4">
            <div className="flex shrink-0 gap-2">
              <Input
                label="Search sources"
                placeholder="Search for movies, shows, music..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                startContent={
                  <IconSearch size={16} className="text-default-400" />
                }
                onKeyDown={(e) => {
                  if (e.key === "Enter") onSearch();
                }}
                className="flex-1"
              />
              <Button color="primary" isLoading={searching} onPress={onSearch}>
                Search
              </Button>
            </div>

            {searchErrors.length > 0 && (
              <Card className="shrink-0 border border-warning" role="alert">
                <CardBody>{searchErrors.map((message, index) => <p key={index}>{message}</p>)}</CardBody>
              </Card>
            )}
            <DataTable
              stateKey="source-search-results"
              emptyContent={hasSearched ? "No releases matched this search." : "Enter a title to search your sources."}
              data={searchResults}
              isLoading={searching}
              columns={columns}
              getRowKey={getReleaseKey}
              defaultSortColumn="seeders"
              defaultSortDirection="desc"
              toolbarQueryPlaceholder="Filter results..."
              removeWrapper
              showItemCount
              fillHeight
              ariaLabel="Search all sources results"
            />
          </div>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
