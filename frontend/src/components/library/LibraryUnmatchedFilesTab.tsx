import { useEffect, useState } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { Spinner } from "@heroui/spinner";
import { Chip } from "@heroui/chip";
import { Tooltip } from "@heroui/tooltip";
import {
  IconCircleCheck,
  IconVideo,
  IconRefresh,
  IconTrash,
  IconLink,
} from "@tabler/icons-react";
import { apolloClient } from "../../lib/graphql/client";
import {
  LibraryUnmatchedMediaFilesTabDocument,
  type LibraryUnmatchedMediaFilesTabQuery,
} from "../../lib/graphql/generated/graphql";
import { sanitizeError } from "../../lib/format";
import { formatBytes } from "../../lib/format";
import { ErrorState } from "../shared";
import { ManualMatchModal } from "./ManualMatchModal";

type MediaFileNode =
  LibraryUnmatchedMediaFilesTabQuery["MediaFiles"]["Edges"][number]["Node"];

interface LibraryUnmatchedFilesTabProps {
  libraryId: string;
  libraryPath: string;
  libraryType: string;
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean;
}

export function LibraryUnmatchedFilesTab({
  libraryId,
  libraryPath,
  libraryType,
  loading: parentLoading,
}: LibraryUnmatchedFilesTabProps) {
  const [files, setFiles] = useState<MediaFileNode[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [matchModalOpen, setMatchModalOpen] = useState(false);
  const [selectedFile, setSelectedFile] = useState<MediaFileNode | null>(null);

  const fetchUnmatchedFiles = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await apolloClient.query({
        query: LibraryUnmatchedMediaFilesTabDocument,
        variables: { LibraryId: libraryId },
        fetchPolicy: "network-only",
      });

      const edges = result.data?.MediaFiles?.Edges ?? [];
      setFiles(edges.map((edge) => edge.Node));
    } catch (err) {
      setError(sanitizeError(err));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchUnmatchedFiles();
  }, [libraryId]);

  const getFileName = (path: string) => {
    const parts = path.split("/");
    return parts[parts.length - 1];
  };

  const getRelativePath = (file: MediaFileNode) => {
    if (file.RelativePath) return file.RelativePath;
    // Try to extract relative path from full path
    if (file.Path.startsWith(libraryPath)) {
      return file.Path.slice(libraryPath.length + 1);
    }
    return file.Path;
  };

  if (isLoading) {
    return (
      <div className="flex justify-center items-center py-12">
        <Spinner size="lg" />
      </div>
    );
  }

  if (error) {
    return (
      <ErrorState
        title="Error Loading Files"
        message={error}
        onRetry={fetchUnmatchedFiles}
      />
    );
  }

  return (
    <div className="space-y-4 w-full">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">Unmatched Files</h2>
          <p className="text-sm text-default-500">
            Files found in the library that couldn't be matched to a show (
            {files.length} files)
          </p>
        </div>
        <Button
          color="primary"
          variant="flat"
          size="sm"
          onPress={fetchUnmatchedFiles}
          isLoading={parentLoading || isLoading}
        >
          <IconRefresh size={16} />
          Refresh
        </Button>
      </div>

      {files.length === 0 ? (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-12 text-center">
            <IconCircleCheck
              size={48}
              className="mx-auto mb-4 text-green-400"
            />
            <h3 className="text-lg font-semibold mb-2">No unmatched files</h3>
            <p className="text-default-500 mb-4">
              All files in this library have been matched to shows.
            </p>
            <p className="text-xs text-default-400">
              Library path:{" "}
              <code className="bg-content2 px-2 py-1 rounded">
                {libraryPath}
              </code>
            </p>
          </CardBody>
        </Card>
      ) : (
        <div className="space-y-2">
          {files.map((file) => (
            <Card key={file.Id} className="bg-content2">
              <CardBody className="py-3">
                <div className="flex items-start gap-3">
                  <div className="p-2 bg-content3 rounded-lg">
                    <IconVideo size={24} className="text-purple-400" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <p
                        className="font-medium truncate"
                        title={getFileName(file.Path)}
                      >
                        {file.OriginalName || getFileName(file.Path)}
                      </p>
                      {file.Resolution && (
                        <Chip size="sm" variant="flat" color="primary">
                          {file.Resolution}
                        </Chip>
                      )}
                      {file.IsHdr && (
                        <Chip size="sm" variant="flat" color="warning">
                          {file.HdrType || "HDR"}
                        </Chip>
                      )}
                      {file.VideoCodec && (
                        <Chip size="sm" variant="flat" color="default">
                          {file.VideoCodec}
                        </Chip>
                      )}
                    </div>
                    <p
                      className="text-xs text-default-400 truncate"
                      title={file.Path}
                    >
                      {getRelativePath(file)}
                    </p>
                    <div className="flex items-center gap-4 mt-2 text-xs text-default-500">
                      <span>{formatBytes(file.Size)}</span>
                      {file.Container && <span>.{file.Container}</span>}
                      {file.AudioCodec && <span>{file.AudioCodec}</span>}
                      {file.Duration && (
                        <span>
                          {Math.floor(file.Duration / 60)}m {file.Duration % 60}
                          s
                        </span>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <Tooltip content="Manually match file">
                      <Button
                        isIconOnly
                        size="sm"
                        variant="light"
                        onPress={() => {
                          setSelectedFile(file);
                          setMatchModalOpen(true);
                        }}
                      >
                        <IconLink size={16} className="text-primary" />
                      </Button>
                    </Tooltip>
                    <Tooltip content="Remove from database (coming soon)">
                      <Button
                        isIconOnly
                        size="sm"
                        variant="light"
                        color="danger"
                        isDisabled
                      >
                        <IconTrash size={16} />
                      </Button>
                    </Tooltip>
                  </div>
                </div>
              </CardBody>
            </Card>
          ))}
        </div>
      )}

      {/* Manual Match Modal */}
      <ManualMatchModal
        isOpen={matchModalOpen}
        onClose={() => {
          setMatchModalOpen(false);
          setSelectedFile(null);
        }}
        mediaFile={selectedFile}
        libraryId={libraryId}
        libraryType={libraryType}
        onMatched={() => {
          fetchUnmatchedFiles();
        }}
      />
    </div>
  );
}
