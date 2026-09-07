import { useEffect, useState } from "react";
import { Button } from "@heroui/button";
import { Chip } from "@heroui/chip";
import {
  Modal,
  ModalBody,
  ModalContent,
  ModalFooter,
  ModalHeader,
} from "@heroui/modal";
import { addToast } from "@heroui/toast";
import {
  IconAlertTriangle,
  IconCheck,
  IconRefresh,
  IconTrash,
} from "@tabler/icons-react";
import { useMutation } from "../lib/graphql/client";
import {
  ResolveScanIssueDocument,
  RetryScanIssueDocument,
  TrashDuplicateScanIssueDocument,
  type ResolveScanIssueMutation,
  type ResolveScanIssueMutationVariables,
  type RetryScanIssueMutation,
  type RetryScanIssueMutationVariables,
  type TrashDuplicateScanIssueMutation,
  type TrashDuplicateScanIssueMutationVariables,
  type UnresolvedLibraryScanIssuesQuery,
} from "../lib/graphql/generated/graphql";
import { sanitizeError } from "../lib/format";

export type ScanIssueNotification =
  UnresolvedLibraryScanIssuesQuery["libraryScanIssues"]["edges"][number]["node"];

interface ScanIssueDetailModalProps {
  issue: ScanIssueNotification | null;
  isOpen: boolean;
  onClose: () => void;
  onChanged: () => void | Promise<void>;
}

interface DuplicateDetails {
  duplicatePath: string;
  keeperPath: string;
  size: number | null;
  storageRelationship: string;
}

function duplicateDetails(issue: ScanIssueNotification): DuplicateDetails | null {
  if (!issue.detailsJson) return null;
  try {
    const value = JSON.parse(issue.detailsJson) as {
      duplicatePath?: unknown;
      keeperPath?: unknown;
      size?: unknown;
      storageRelationship?: unknown;
    };
    if (
      typeof value.duplicatePath !== "string" ||
      typeof value.keeperPath !== "string"
    ) {
      return null;
    }
    return {
      duplicatePath: value.duplicatePath,
      keeperPath: value.keeperPath,
      size: typeof value.size === "number" ? value.size : null,
      storageRelationship:
        typeof value.storageRelationship === "string"
          ? value.storageRelationship
          : "UNKNOWN",
    };
  } catch {
    return null;
  }
}

function formatCode(value: string): string {
  return value
    .toLowerCase()
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function ScanIssueDetailModal({
  issue,
  isOpen,
  onClose,
  onChanged,
}: ScanIssueDetailModalProps) {
  const [confirmingTrash, setConfirmingTrash] = useState(false);
  const [retryIssue, { loading: retrying }] = useMutation<
    RetryScanIssueMutation,
    RetryScanIssueMutationVariables
  >(RetryScanIssueDocument);
  const [resolveIssue, { loading: resolving }] = useMutation<
    ResolveScanIssueMutation,
    ResolveScanIssueMutationVariables
  >(ResolveScanIssueDocument);
  const [trashDuplicate, { loading: trashing }] = useMutation<
    TrashDuplicateScanIssueMutation,
    TrashDuplicateScanIssueMutationVariables
  >(TrashDuplicateScanIssueDocument);

  useEffect(() => {
    setConfirmingTrash(false);
  }, [issue?.id, isOpen]);

  if (!issue) return null;

  const duplicate = duplicateDetails(issue);
  const canRetry = issue.stage === "ANALYSIS" && Boolean(issue.mediaFileId);
  const isDuplicate =
    issue.issueCode === "BYTE_IDENTICAL_DUPLICATE" && duplicate !== null;
  const busy = retrying || resolving || trashing;

  const handleRetry = async () => {
    try {
      const result = await retryIssue({ variables: { issueId: issue.id } });
      const action = result.data?.retryScanIssue;
      addToast({
        title: action?.success ? "Analysis retry requested" : "Retry failed",
        description: action?.message ?? "The retry returned no result.",
        color: action?.success ? "success" : "danger",
      });
      await onChanged();
    } catch (error) {
      addToast({
        title: "Retry failed",
        description: sanitizeError(error),
        color: "danger",
      });
    }
  };

  const handleResolve = async () => {
    try {
      const result = await resolveIssue({
        variables: {
          issueId: issue.id,
          resolution: "Reviewed and acknowledged by an administrator.",
        },
      });
      const action = result.data?.resolveScanIssue;
      addToast({
        title: action?.success ? "Issue reviewed" : "Review failed",
        description: action?.message ?? "The action returned no result.",
        color: action?.success ? "success" : "danger",
      });
      await onChanged();
      if (action?.success) onClose();
    } catch (error) {
      addToast({
        title: "Review failed",
        description: sanitizeError(error),
        color: "danger",
      });
    }
  };

  const handleTrash = async () => {
    try {
      const result = await trashDuplicate({
        variables: { issueId: issue.id },
      });
      const action = result.data?.trashDuplicateScanIssue;
      addToast({
        title: action?.success ? "Duplicate moved to trash" : "File unchanged",
        description: action?.message ?? "The action returned no result.",
        color: action?.success ? "success" : "danger",
      });
      await onChanged();
      if (action?.success) onClose();
    } catch (error) {
      addToast({
        title: "File unchanged",
        description: sanitizeError(error),
        color: "danger",
      });
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="2xl"
      scrollBehavior="inside"
    >
      <ModalContent>
        <ModalHeader className="flex items-center gap-3">
          <IconAlertTriangle size={20} className="shrink-0 text-warning" />
          <span className="min-w-0 flex-1 truncate">
            {formatCode(issue.issueCode)}
          </span>
        </ModalHeader>
        <ModalBody className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <Chip size="sm" variant="flat" color="warning">
              {issue.severity}
            </Chip>
            <Chip size="sm" variant="flat">
              {formatCode(issue.stage)}
            </Chip>
            {issue.occurrenceCount > 1 ? (
              <Chip size="sm" variant="flat">
                {issue.occurrenceCount} occurrences
              </Chip>
            ) : null}
          </div>

          <p className="break-words text-sm">{issue.message}</p>
          {issue.remediation ? (
            <p className="rounded-medium bg-default-100 p-3 text-sm text-default-600">
              {issue.remediation}
            </p>
          ) : null}

          {duplicate ? (
            <dl className="space-y-3 rounded-medium border border-default-200 p-3 text-xs">
              <div>
                <dt className="font-medium">Keep</dt>
                <dd className="break-all text-default-500">
                  {duplicate.keeperPath}
                </dd>
              </div>
              <div>
                <dt className="font-medium">Move to trash</dt>
                <dd className="break-all text-default-500">
                  {duplicate.duplicatePath}
                </dd>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <dt className="font-medium">Storage relationship</dt>
                  <dd className="text-default-500">
                    {duplicate.storageRelationship.toLowerCase()}
                  </dd>
                </div>
                <div>
                  <dt className="font-medium">Size</dt>
                  <dd className="text-default-500">
                    {duplicate.size === null
                      ? "Unknown"
                      : `${duplicate.size.toLocaleString()} bytes`}
                  </dd>
                </div>
              </div>
            </dl>
          ) : null}

          {confirmingTrash ? (
            <p className="rounded-medium border border-danger-300/40 bg-danger-50/10 p-3 text-sm">
              Librarian will re-hash both files, move only the verified
              duplicate into the recoverable <code>.librarian-trash</code>{" "}
              folder, and remove its database row. This does not permanently
              delete the file.
            </p>
          ) : null}
        </ModalBody>
        <ModalFooter className="flex-wrap">
          <Button variant="light" onPress={onClose} isDisabled={busy}>
            Close
          </Button>
          {canRetry ? (
            <Button
              variant="flat"
              startContent={<IconRefresh size={16} />}
              onPress={() => void handleRetry()}
              isLoading={retrying}
              isDisabled={resolving || trashing}
            >
              Retry analysis
            </Button>
          ) : null}
          <Button
            variant="flat"
            startContent={<IconCheck size={16} />}
            onPress={() => void handleResolve()}
            isLoading={resolving}
            isDisabled={retrying || trashing}
          >
            Mark reviewed
          </Button>
          {isDuplicate ? (
            <Button
              color="danger"
              variant={confirmingTrash ? "solid" : "flat"}
              startContent={<IconTrash size={16} />}
              onPress={() =>
                confirmingTrash
                  ? void handleTrash()
                  : setConfirmingTrash(true)
              }
              isLoading={trashing}
              isDisabled={retrying || resolving}
            >
              {confirmingTrash ? "Confirm move to trash" : "Move to trash"}
            </Button>
          ) : null}
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
