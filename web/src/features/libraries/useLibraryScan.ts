import { useMutation, useQuery, useSubscription } from "@apollo/client/react";
import { toast } from "@heroui/react";

import { EntityLibraryScanRunChangedDocument, LibraryScanStateDocument, ScanLibraryDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

/** Latest scan run for a library plus a `scan()` action; live-updates from the run subscription. */
export function useLibraryScan(libraryId: string) {
  const state = useQuery(LibraryScanStateDocument, { variables: { libraryId } });
  useSubscription(EntityLibraryScanRunChangedDocument, {
    onData: ({ data }) => {
      if (data.data?.libraryScanRunChanged.libraryScanRun?.libraryId === libraryId) void state.refetch();
    },
  });
  const [scanLibrary, { loading }] = useMutation(ScanLibraryDocument);

  const scan = async () => {
    try {
      const { data } = await scanLibrary({ variables: { id: libraryId } });
      const result = data?.scanLibrary;
      if (result?.success) toast.success(result.message ?? "Scan started");
      else toast.warning(result?.message ?? "Scan could not start");
      void state.refetch();
    } catch (error) {
      toast.danger(errorMessage(error, "Scan could not start"));
    }
  };

  const latestRun = state.data?.libraryScanRuns.edges[0]?.node ?? null;
  const running = latestRun?.status === "RUNNING" || latestRun?.status === "QUEUED";
  return { latestRun, running, unresolvedIssues: state.data?.libraryScanIssues.pageInfo.totalCount ?? 0, scan, scanning: loading };
}
