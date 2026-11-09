import { useMutation } from "@apollo/client/react";
import { IconDownload } from "glass-ux/icons";
import { Badge } from "glass-ux/badge";
import { toast } from "glass-ux/toast";
import { Tooltip } from "glass-ux/tooltip";
import { useEffect, useState } from "react";

import { Button, ConfirmDialog } from "@/components/ui";
import { EntityReleaseRecommendationListDocument, GrabRecommendationDocument, GrabRecommendationPlanDocument, LookupRecommendationsDocument } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery } from "@/lib/graphql/live/hooks";

import { addsLabel, coverageLabel, lookupInput, planSummary, planView, recommendationFilter, scopeLabel, targetKey, visibleRows, type Recommendation, type RecommendationTarget } from "./recommendations";

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
 * one-press Download. For a show it is a coverage plan: the fewest releases that cover what is
 * missing (packs where they exist, episodes for the gaps), with "Download all" and what stays
 * uncovered; a season shows its part of the plan. Opening a page that has missing items asks the
 * server to look once; the answer appears when it has (live), and disappears when the release is
 * gone or the item is filled. Administrators only, like the other ways of finding and grabbing a
 * release.
 */
export function RecommendedRelease({ target, missing, quiet = false }: RecommendedReleaseProps) {
  const isAdmin = useIsAdmin();
  const filter = recommendationFilter(target);
  const key = targetKey(target);
  const list = useLiveQuery(EntityReleaseRecommendationListDocument, {
    variables: { where: filter, orderBy: [{ foundAt: "DESC" }], page: { limit: target.showId ? 200 : 60, offset: 0 } },
    skip: !filter || missing <= 0 || !isAdmin,
  });
  const rows = list.data?.releaseRecommendations.edges.map((edge) => edge.node) ?? [];
  const [lookup] = useMutation(LookupRecommendationsDocument);
  const loaded = Boolean(list.data) && !list.loading;
  // A show is answered once its plan exists; rows of the old per-season kind do not count.
  const answered = target.showId ? rows.some((row) => row.scope === "plan") : rows.length > 0;

  useEffect(() => {
    const input = lookupInput(target);
    if (!isAdmin || missing <= 0 || !loaded || answered || !input || asked.has(key)) return;
    asked.add(key);
    void lookup({ variables: { input } }).catch(() => asked.delete(key));
    // `target` is covered by `key`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAdmin, missing, loaded, answered, key, lookup]);

  if (!isAdmin || missing <= 0 || !filter) return null;
  const refresh = () => void list.refetch();
  if (target.showId) return <ShowPlan rows={rows} season={target.series ? undefined : target.season} full={Boolean(target.series)} quiet={quiet} onGrabbed={refresh} />;

  const shown = visibleRows(rows);
  if (shown.ready.length === 0) {
    if (shown.searching) return <Looking />;
    if (shown.none && !quiet) return <NothingYet note={shown.none.note} />;
    return null;
  }
  const lines = shown.ready.slice(0, MAX_LINES);
  return (
    <div className="flex min-w-0 flex-col gap-1.5" aria-label="Recommended releases">
      {lines.map((row) => <RecommendationLine key={row.id} row={row} onGrabbed={refresh} />)}
    </div>
  );
}

function Looking() {
  return <p className="text-label-sm text-muted" role="status">Looking for the best release…</p>;
}

function NothingYet({ note }: { note: string | null | undefined }) {
  return <p className="text-label-sm text-muted" role="status">No suitable release found yet{note ? `: ${note}` : ""}</p>;
}

/**
 * A show's coverage plan. `full` shows all of it with its summary, "Download all" and what is still
 * uncovered; otherwise only the releases covering `season`, which link back to the show's plan.
 */
function ShowPlan({ rows, season, full, quiet, onGrabbed }: { rows: readonly Recommendation[]; season?: number; full: boolean; quiet: boolean; onGrabbed: () => void }) {
  const view = planView(rows, season);
  const plan = view.plan;
  if (!plan) {
    if (view.searching && (full || quiet)) return <Looking />;
    if (view.none && full && !quiet) return <NothingYet note={view.none.note} />;
    return null;
  }
  const lines = full ? view.releases : view.releases.slice(0, MAX_LINES);
  const hidden = view.releases.length - lines.length;
  return (
    <section className="flex min-w-0 flex-col gap-1.5" aria-label="Recommended releases">
      {full ? <PlanHeader plan={plan} releases={view.releases} onGrabbed={onGrabbed} /> : null}
      <ul className={full ? "flex min-w-0 flex-col gap-1.5 ps-3" : "flex min-w-0 flex-col gap-1.5"}>
        {lines.map((row) => <li key={row.id} className="min-w-0"><RecommendationLine row={row} inPlan onGrabbed={onGrabbed} /></li>)}
      </ul>
      {hidden > 0 ? <p className="text-label-sm text-muted">{hidden} more in this season. The show's plan lists them all.</p> : null}
      {full && plan.note ? <p className="text-label-sm text-muted">{plan.note}</p> : null}
    </section>
  );
}

function PlanHeader({ plan, releases, onGrabbed }: { plan: Recommendation; releases: Recommendation[]; onGrabbed: () => void }) {
  const [grabPlan, { loading }] = useMutation(GrabRecommendationPlanDocument);
  const [confirming, setConfirming] = useState(false);
  const total = releases.reduce((sum, row) => sum + (row.size ?? 0), 0);
  const download = async () => {
    const result = (await grabPlan({ variables: { id: plan.id } })).data?.grabRecommendationPlan;
    if (!result || result.started === 0) throw new Error(result?.failed[0] ?? result?.error ?? "Could not start the downloads");
    if (result.failed.length > 0) toast.warning(`Downloading ${result.started} of ${result.started + result.failed.length} releases. ${result.failed.join(" · ")}`);
    else toast.success(`Downloading ${result.started} ${result.started === 1 ? "release" : "releases"}`);
    setConfirming(false);
    onGrabbed();
  };
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
      <Badge size="sm" tone="accent">Recommended</Badge>
      <Tooltip content={plan.reasons.length ? plan.reasons.join(" · ") : "The fewest releases at the best quality your profile accepts"}>
        <span className="min-w-0 flex-1 basis-64 truncate text-body-sm text-foreground" tabIndex={0}>
          {planSummary(plan, releases.length)}
          <span className="text-muted">{total > 0 ? ` · ${formatBytes(total)}` : ""}</span>
        </span>
      </Tooltip>
      {releases.length > 1 ? (
        <Button size="sm" variant="primary" isPending={loading} onPress={() => setConfirming(true)} aria-label={`Download all ${releases.length} recommended releases`}>
          <IconDownload size="sm" /> Download all
        </Button>
      ) : null}
      <ConfirmDialog
        isOpen={confirming}
        onOpenChange={setConfirming}
        title={`Download ${releases.length} releases?`}
        description={`Each release starts its own download${total > 0 ? `, ${formatBytes(total)} in all` : ""}. With "Download only the missing files" on, packs take only the files for what is missing.`}
        confirmLabel="Download all"
        isPending={loading}
        onConfirm={download}
      />
    </div>
  );
}

function RecommendationLine({ row, inPlan = false, onGrabbed }: { row: Recommendation; inPlan?: boolean; onGrabbed: () => void }) {
  const [grab, { loading }] = useMutation(GrabRecommendationDocument);
  const coverage = inPlan ? addsLabel(row) : coverageLabel(row);
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
  // In a plan the release says what it is for (`Season 2`, `S03E05`); alone, its scope.
  const what = inPlan ? row.targetLabel.replace(/^.*? (?=(Season|Seasons|Complete series|S\d{2}E\d{2}))/, "") : `${scopeLabel(row.scope)}${row.scope === "episode" ? ` ${row.targetLabel}` : ""}`;
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
      {inPlan ? null : <Badge size="sm" tone="accent">Recommended</Badge>}
      <Tooltip content={row.reasons.length ? row.reasons.join(" · ") : "The best release your quality profile accepts"}>
        <span className="min-w-0 flex-1 basis-64 truncate text-body-sm text-foreground" tabIndex={0}>
          <span className="text-muted">{what} · </span>
          {row.title}
          <span className="text-muted">{details ? ` · ${details}` : ""}{source ? ` · ${source}` : ""}</span>
        </span>
      </Tooltip>
      <Button size="sm" variant={inPlan ? "secondary" : "primary"} isPending={loading} onPress={() => void download()} aria-label={`Download ${row.title ?? "the recommended release"}`}>
        <IconDownload size="sm" /> Download
      </Button>
    </div>
  );
}
