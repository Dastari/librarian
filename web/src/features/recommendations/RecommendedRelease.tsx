import { useMutation } from "@apollo/client/react";
import { IconDownload } from "glass-ux/icons";
import { Badge } from "glass-ux/badge";
import { toast } from "glass-ux/toast";
import { Tooltip } from "glass-ux/tooltip";
import { useEffect } from "react";

import { Button } from "@/components/ui";
import { EntityReleaseRecommendationListDocument, GrabRecommendationDocument, LookupRecommendationsDocument } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery } from "@/lib/graphql/live/hooks";

import { coverageLabel, lookupInput, recommendationFilter, scopeLabel, targetKey, visibleRows, type Recommendation, type RecommendationTarget } from "./recommendations";

/** Lookups already asked for in this session, so opening a page again does not ask again. */
const asked = new Set<string>();

/** Most recommendation lines shown under one season before the rest are summarised. */
const MAX_LINES = 3;

interface RecommendedReleaseProps {
  target: RecommendationTarget;
  /** How many items are missing under this target. Nothing is shown, or looked up, when none are. */
  missing: number;
  /** A pinned line (the Find releases dialog) leaves out the "nothing found" note. */
  quiet?: boolean;
}

/**
 * "Recommended": the best release for what is missing here, worked out ahead of time, with a
 * one-press Download. Opening a page that has missing items asks the server to look once; the line
 * appears when it has (live), and disappears when the release is gone or the item is filled.
 * Administrators only, like the other ways of finding and grabbing a release.
 */
export function RecommendedRelease({ target, missing, quiet = false }: RecommendedReleaseProps) {
  const isAdmin = useIsAdmin();
  const filter = recommendationFilter(target);
  const key = targetKey(target);
  const list = useLiveQuery(EntityReleaseRecommendationListDocument, {
    variables: { where: filter, orderBy: [{ foundAt: "DESC" }], page: { limit: 60, offset: 0 } },
    skip: !filter || missing <= 0 || !isAdmin,
  });
  const rows = list.data?.releaseRecommendations.edges.map((edge) => edge.node) ?? [];
  const [lookup] = useMutation(LookupRecommendationsDocument);
  const loaded = Boolean(list.data) && !list.loading;

  useEffect(() => {
    const input = lookupInput(target);
    if (!isAdmin || missing <= 0 || !loaded || rows.length > 0 || !input || asked.has(key)) return;
    asked.add(key);
    void lookup({ variables: { input } }).catch(() => asked.delete(key));
    // `target` is covered by `key`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAdmin, missing, loaded, rows.length, key, lookup]);

  if (!isAdmin || missing <= 0 || !filter) return null;
  const shown = visibleRows(rows);
  if (shown.ready.length === 0) {
    if (shown.searching) return <p className="text-label-sm text-muted" role="status">Looking for the best release…</p>;
    if (shown.none && !quiet) return <p className="text-label-sm text-muted" role="status">No suitable release found yet{shown.none.note ? `: ${shown.none.note}` : ""}</p>;
    return null;
  }
  const lines = shown.ready.slice(0, MAX_LINES);
  return (
    <div className="flex min-w-0 flex-col gap-1.5" aria-label="Recommended releases">
      {lines.map((row) => <RecommendationLine key={row.id} row={row} onGrabbed={() => void list.refetch()} />)}
      {shown.ready.length > lines.length ? <p className="text-label-sm text-muted">{shown.ready.length - lines.length} more recommended in this season. Use Find on an episode to see them.</p> : null}
    </div>
  );
}

function RecommendationLine({ row, onGrabbed }: { row: Recommendation; onGrabbed: () => void }) {
  const [grab, { loading }] = useMutation(GrabRecommendationDocument);
  const coverage = coverageLabel(row);
  const details = [row.size ? formatBytes(row.size) : null, row.seeders !== null && row.seeders !== undefined ? `${row.seeders} seeders` : null, row.quality, coverage].filter(Boolean).join(" · ");
  const source = row.sourceKind === "feed" ? `from feed ${row.sourceName ?? ""}`.trim() : row.sourceName ? `from ${row.sourceName}` : null;
  const download = async () => {
    try {
      const result = assertSuccess((await grab({ variables: { id: row.id } })).data?.grabRecommendation, "Could not start the download");
      toast.success(`Downloading ${result.torrentName ?? row.title ?? "the release"}`);
      onGrabbed();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
      <Badge size="sm" tone="accent">Recommended</Badge>
      <Tooltip content={row.reasons.length ? row.reasons.join(" · ") : "The best release your quality profile accepts"}>
        <span className="min-w-0 flex-1 basis-64 truncate text-body-sm text-foreground" tabIndex={0}>
          <span className="text-muted">{scopeLabel(row.scope)}{row.scope === "episode" ? ` ${row.targetLabel}` : ""} · </span>
          {row.title}
          <span className="text-muted">{details ? ` · ${details}` : ""}{source ? ` · ${source}` : ""}</span>
        </span>
      </Tooltip>
      <Button size="sm" variant="primary" isPending={loading} onPress={() => void download()} aria-label={`Download ${row.title ?? "the recommended release"}`}>
        <IconDownload size="sm" /> Download
      </Button>
    </div>
  );
}
