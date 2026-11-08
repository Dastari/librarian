import { useLazyQuery, useMutation } from "@apollo/client/react";
import { defineActions } from "glass-ux/actions";
import { ActivityFeed, type ActivityItem } from "glass-ux/activity-feed";
import { Badge } from "glass-ux/badge";
import { ButtonLink } from "glass-ux/button-link";
import { createDataTableColumns, DataTable, TextCell } from "glass-ux/data-table";
import { IconCheck, IconClock, IconInfo, IconRefresh, IconSearch } from "glass-ux/icons";
import { ScrollArea } from "glass-ux/scroll-area";
import { Select } from "glass-ux/select";
import { TruncatedText } from "glass-ux/truncated-text";
import { Link } from "@tanstack/react-router";
import { toast } from "glass-ux/toast";
import { createContext, useContext, useMemo, useState } from "react";

import { Artwork, Button, ConfirmDialog, EmptyState, Spinner } from "@/components/ui";
import {
  IdentificationCandidatesDocument,
  IdentifyTorrentDocument,
  ImportIdentifiedItemDocument,
  TorrentIdentificationDocument,
  type TorrentIdentificationQuery,
} from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { entityPath } from "@/lib/entities/kinds";
import { formatRuntime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { ChooseListingDialog, type Listing } from "./ChooseListingDialog";

export type IdentifiedItem = TorrentIdentificationQuery["identifiedItems"]["edges"][number]["node"];
type Item = IdentifiedItem;
type Event = TorrentIdentificationQuery["identificationEvents"]["edges"][number]["node"];

export const IDENTIFIED_STATUS: Record<string, { label: string; tone: "success" | "info" | "warning" | "neutral"; order: number }> = {
  needs_choice: { label: "Needs a choice", tone: "warning", order: 0 },
  wanted: { label: "Wanted", tone: "info", order: 1 },
  adds: { label: "Adds to library", tone: "info", order: 2 },
  upgrade: { label: "Upgrade available", tone: "success", order: 3 },
  identified: { label: "Ready to add", tone: "info", order: 4 },
  in_library: { label: "In library", tone: "neutral", order: 5 },
  unknown: { label: "Not identified", tone: "warning", order: 6 },
  imported: { label: "Imported", tone: "success", order: 7 },
};

/** What one place in a title is called, by kind. */
const PLACE: Record<string, [string, string]> = { music: ["track", "tracks"], tv: ["episode", "episodes"] };

/** A result's status in words, counting what it adds where that's countable. */
export function statusLabel(item: Pick<Item, "status" | "kind" | "addsCount">): string {
  const place = PLACE[item.kind];
  const count = item.addsCount ?? 0;
  if (item.status === "adds") {
    if (place && count > 0) return `Adds ${count} ${count === 1 ? place[0] : place[1]}`;
    return item.kind === "movie" ? "Adds the film" : item.kind === "audiobook" ? "Adds the book" : IDENTIFIED_STATUS.adds!.label;
  }
  return (IDENTIFIED_STATUS[item.status] ?? IDENTIFIED_STATUS.unknown!).label;
}

const KIND: Record<string, string> = { audiobook: "Audiobook", music: "Music", tv: "TV", movie: "Movie", ebook: "Ebook", unknown: "Unknown" };
const ENTITY: Record<string, "audiobook" | "album" | "show" | "movie"> = { Audiobook: "audiobook", Album: "album", Show: "show", Movie: "movie" };

export interface Finding {
  source: string;
  text: string;
  warning: boolean;
  short?: string;
}

/** What identification found out, each with where it came from. Older results are plain sentences. */
export function parseFindings(value: string): Finding[] {
  try {
    const parsed: unknown = JSON.parse(value);
    if (!Array.isArray(parsed)) return [];
    return parsed.flatMap((entry: unknown): Finding[] => {
      if (typeof entry === "string") return [{ source: "other", text: entry, warning: false }];
      const finding = entry as Partial<Finding> | null;
      return finding && typeof finding.text === "string" ? [{ source: finding.source ?? "other", text: finding.text, warning: Boolean(finding.warning), short: finding.short }] : [];
    });
  } catch { return []; }
}

const FINDING_GROUPS = [
  { source: "files", title: "In the files" },
  { source: "heard", title: "Heard in the opening" },
  { source: "listing", title: "Online listing" },
  { source: "library", title: "In your library" },
  { source: "jev", title: "Jev" },
  { source: "other", title: "What was found" },
];

/** The narrator named in a download's tags or text files, if any. */
function narratorOf(item: Item): string | null {
  try {
    const narrators = (JSON.parse(item.identity) as { narrators?: unknown }).narrators;
    return Array.isArray(narrators) && typeof narrators[0] === "string" ? narrators[0] : null;
  } catch { return null; }
}

/** Rows store their times as Unix seconds. */
const atTime = (value: string) => (/^\d+$/.test(value) ? Number(value) * 1000 : value);

function seconds(ms: number) {
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(ms < 10000 ? 1 : 0)} s`;
}

export function useTorrentIdentification(infoHash: string) {
  const { data, loading, error, refetch } = useQuery(TorrentIdentificationDocument, { variables: { infoHash } });
  const items = useMemo(() => {
    const nodes = data?.identifiedItems.edges.map((edge) => edge.node) ?? [];
    const rank = (item: Item) => IDENTIFIED_STATUS[item.status]?.order ?? 9;
    return [...nodes].sort((a, b) => rank(a) - rank(b) || (a.title ?? a.label).localeCompare(b.title ?? b.label));
  }, [data]);
  const events = useMemo(() => data?.identificationEvents.edges.map((edge) => edge.node) ?? [], [data]);
  return { items, events, loading: loading && !data, error, refetch };
}

export type TorrentIdentificationState = ReturnType<typeof useTorrentIdentification>;

function RunAgain({ infoHash, onDone }: { infoHash: string; onDone: () => void }) {
  const [run, { loading }] = useMutation(IdentifyTorrentDocument);
  return (
    <Button size="sm" variant="ghost" isPending={loading} onPress={() => void run({ variables: { infoHash } }).then(({ data }) => {
      assertSuccess(data?.identifyTorrent, "Could not start identifying this download");
      toast.success("Looking at this download again");
      onDone();
    }).catch((error: unknown) => toast.danger(errorMessage(error)))}>
      <IconRefresh size="md" /> Run again
    </Button>
  );
}

const TREAT_AS = [
  { value: "auto", label: "Work it out" },
  { value: "audiobook", label: "Audiobook" },
  { value: "music", label: "Music" },
  { value: "tv", label: "TV" },
  { value: "movie", label: "Movie" },
];

/** For when identification has the wrong idea of what a download is. */
function TreatAs({ infoHash, kind, onDone }: { infoHash: string; kind: string | null | undefined; onDone: () => void }) {
  const [run, { loading }] = useMutation(IdentifyTorrentDocument);
  return (
    <Select
      aria-label="Treat this download as"
      size="sm"
      value={kind ?? "auto"}
      disabled={loading}
      options={TREAT_AS}
      className="w-40"
      onValueChange={(value) => {
        if (!value) return;
        void run({ variables: { infoHash, kind: value } }).then(({ data }) => {
          assertSuccess(data?.identifyTorrent, "Could not change what this is treated as");
          onDone();
        }).catch((error: unknown) => toast.danger(errorMessage(error)));
      }}
    />
  );
}

export interface ResultsActions {
  canImport: boolean;
  busy: string | null;
  importIt: (item: Item, options?: { acceptReview?: boolean; betterCopies?: boolean }) => void;
  choose: (item: Item) => void;
  reviewUpgrade: (item: Item) => void;
}

export const ResultsContext = createContext<ResultsActions>({ canImport: false, busy: null, importIt: () => {}, choose: () => {}, reviewUpgrade: () => {} });

/** The button for adding what a result fills in. */
function addLabel(item: Item) {
  if (item.kind === "music") return "Add missing tracks";
  if (item.kind === "tv") return "Add episodes";
  return "Add to library";
}

const finished = (item: Item) => item.finishedFiles >= item.fileCount;
const entityOf = (item: Item) => item.libraryItemType && item.libraryItemId ? ENTITY[item.libraryItemType] : undefined;

export function TitleCell({ item }: { item: Item }) {
  const entity = entityOf(item);
  const title = item.title ?? item.label;
  const narrator = narratorOf(item);
  const facts = [
    item.kind === "audiobook" ? null : KIND[item.kind],
    item.creator,
    narrator ? `read by ${narrator}` : null,
    item.year,
    item.season != null ? `Season ${item.season}` : null,
  ].filter(Boolean).join(" · ");
  return (
    <div className="flex min-w-0 items-center gap-3 py-1">
      <Artwork src={item.coverUrl} alt="" aspect="square" className="w-10 shrink-0" />
      <div className="min-w-0">
        {entity && item.libraryItemId
          ? <Link to={entityPath(entity, item.libraryItemId)} className="block min-w-0 hover:underline"><TruncatedText className="block min-w-0 text-body-sm font-medium text-foreground">{title}</TruncatedText></Link>
          : <TruncatedText className="block min-w-0 text-body-sm font-medium text-foreground">{title}</TruncatedText>}
        {facts ? <TruncatedText className="block min-w-0 text-label-sm text-muted">{facts}</TruncatedText> : null}
      </div>
    </div>
  );
}

export function StatusCell({ item }: { item: Item }) {
  const status = IDENTIFIED_STATUS[item.status] ?? IDENTIFIED_STATUS.unknown!;
  const warnings = [...new Set(parseFindings(item.evidence).flatMap((finding) => finding.short ? [finding.short] : []))];
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-1.5">
      <Badge size="sm" marker="dot" tone={status.tone}>{statusLabel(item)}</Badge>
      {warnings.map((warning) => <Badge key={warning} size="sm" tone="warning">{warning}</Badge>)}
    </div>
  );
}

/** The one thing to do next with a result. Anything else is in its row's menu. */
export function ItemAction({ item }: { item: Item }) {
  const { canImport, busy, importIt, choose, reviewUpgrade } = useContext(ResultsContext);
  const entity = entityOf(item);
  // Everything here is already held: there's nothing to do but look at it.
  if ((item.status === "imported" || item.status === "in_library") && entity && item.libraryItemId) {
    return <ButtonLink size="sm" variant="ghost" render={<Link to={entityPath(entity, item.libraryItemId)} />}>Open</ButtonLink>;
  }
  if (!canImport || !finished(item)) return null;
  const pending = busy === item.id;
  if (item.status === "identified") return <Button size="sm" variant="primary" isPending={pending} onPress={() => importIt(item)}>Add to library</Button>;
  if (item.status === "adds") return <Button size="sm" variant="primary" isPending={pending} onPress={() => importIt(item)}>{addLabel(item)}</Button>;
  if (item.status === "upgrade") return <Button size="sm" variant="secondary" isPending={pending} onPress={() => reviewUpgrade(item)}>Review upgrade</Button>;
  if (item.status === "wanted") return <Button size="sm" variant="primary" isPending={pending} onPress={() => importIt(item)}>Import</Button>;
  if (item.status === "needs_choice" || item.status === "unknown") return <Button size="sm" variant="primary" onPress={() => choose(item)}>Choose</Button>;
  return null;
}

function Finding({ finding }: { finding: Finding }) {
  if (finding.text.startsWith("\u201c")) return <p className="text-body-sm italic text-foreground [overflow-wrap:anywhere]">{finding.text}</p>;
  return (
    <div className="flex min-w-0 items-start gap-2 text-body-sm text-foreground">
      <span className={`mt-0.5 shrink-0 ${finding.warning ? "text-warning" : "text-success"}`}>{finding.warning ? <IconInfo size="sm" /> : <IconCheck size="sm" />}</span>
      <span className="min-w-0 [overflow-wrap:anywhere]">{finding.text}</span>
    </div>
  );
}

/** Why a result is thought to be what it is, grouped by where each thing came from. */
export function ItemFindings({ item }: { item: Item }) {
  const { canImport, busy, importIt } = useContext(ResultsContext);
  const findings = parseFindings(item.evidence);
  const groups = FINDING_GROUPS.flatMap((group) => {
    const lines = findings.filter((finding) => finding.source === group.source);
    return lines.length ? [{ ...group, lines }] : [];
  });
  const named = item.title && item.title !== item.label ? `Files say: ${item.label}` : null;
  const summary = item.status === "imported" ? null : item.summary;
  return (
    <div className="flex min-w-0 flex-col gap-3">
      {named || summary ? <p className="text-label-sm text-muted [overflow-wrap:anywhere]">{[named, summary].filter(Boolean).join(" · ")}</p> : null}
      {groups.length ? (
        <div className="grid min-w-0 grid-cols-[repeat(auto-fit,minmax(15rem,1fr))] gap-x-8 gap-y-4">
          {groups.map((group) => (
            <section key={group.source} className="flex min-w-0 flex-col gap-2">
              <h4 className="text-label-sm font-medium text-muted">{group.title}</h4>
              {group.lines.map((finding) => <Finding key={finding.text} finding={finding} />)}
            </section>
          ))}
        </div>
      ) : null}
      {canImport && item.canAccept && finished(item) ? <div><Button size="sm" variant="secondary" isPending={busy === item.id} onPress={() => importIt(item, { acceptReview: true })}>Import anyway</Button></div> : null}
    </div>
  );
}

/** A result that's settled can still be wrong; one that isn't already asks what it is. */
export const canChooseAnother = (item: Item) => !["imported", "needs_choice", "unknown"].includes(item.status) && item.kind !== "ebook";

const column = createDataTableColumns<Item>();
const COLUMNS = column.columns([
  column.accessor((item) => item.title ?? item.label, { id: "title", header: "Title", width: 26, priority: 1, cardRole: "title", cell: ({ row }) => <TitleCell item={row.original} /> }),
  column.display({ id: "status", header: "Status", width: 16, priority: 1, cell: ({ row }) => <StatusCell item={row.original} /> }),
  column.accessor((item) => item.seconds ?? 0, { id: "length", header: "Length", width: 7, priority: 3, align: "end", cell: ({ row }) => <TextCell muted>{row.original.seconds ? formatRuntime(Math.round(row.original.seconds)) : ""}</TextCell> }),
  column.accessor((item) => item.fileCount, { id: "files", header: "Files", width: 6, priority: 4, align: "end", cell: ({ row }) => <TextCell muted>{finished(row.original) ? row.original.fileCount : `${row.original.finishedFiles} of ${row.original.fileCount}`}</TextCell> }),
  column.display({ id: "details", header: "", width: 3, priority: 1, cell: () => null }),
  column.display({ id: "next", header: "", width: 14, priority: 1, align: "end", cell: ({ row }) => <ItemAction item={row.original} /> }),
]);

const getItemId = (item: Item) => item.id;
const getItemLabel = (item: Item) => item.title ?? item.label;
const NOUN = { one: "title", other: "titles" };
const renderItemFindings = (item: Item) => <ItemFindings item={item} />;

/** What was found in a download: what's already in, what can be added, and what needs deciding. */
export function IdentifiedResults({ infoHash, kind, state, onChanged }: { infoHash: string; kind?: string | null; state: TorrentIdentificationState; onChanged?: () => void }) {
  const isAdmin = useIsAdmin();
  const [choosing, setChoosing] = useState<Item | null>(null);
  const [upgrading, setUpgrading] = useState<Item | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [search, candidates] = useLazyQuery(IdentificationCandidatesDocument, { fetchPolicy: "network-only" });
  const [importItem] = useMutation(ImportIdentifiedItemDocument);
  const refresh = () => { void state.refetch(); onChanged?.(); };
  const results = useMemo<ResultsActions>(() => ({
    canImport: isAdmin,
    busy,
    choose: setChoosing,
    reviewUpgrade: setUpgrading,
    importIt: (item, { acceptReview = false, betterCopies = false } = {}) => {
      setBusy(item.id);
      void importItem({ variables: { input: { itemId: item.id, acceptReview, ...(betterCopies ? { betterCopies } : {}) } } }).then(({ data }) => {
        const result = assertSuccess(data?.importIdentifiedItem, "Could not import this");
        if (result.matchCount === 0) toast.info("Nothing to add: your library already has these");
        else toast.success(`Imported ${result.matchCount} ${result.matchCount === 1 ? "file" : "files"}`);
        void state.refetch();
        onChanged?.();
      }).catch((error: unknown) => toast.danger(errorMessage(error))).finally(() => setBusy(null));
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }), [isAdmin, busy, importItem, state.refetch, onChanged]);
  const actions = useMemo(() => defineActions<Item>("identified-item", [
    {
      id: "something-else", label: "Something else…", icon: IconSearch, group: "manage", priority: 1,
      visible: (rows) => rows.length === 1 && canChooseAnother(rows[0]!),
      run: (rows) => setChoosing(rows[0]!),
    },
  ]), []);
  const choose = async (listing: Listing) => {
    if (!choosing) return;
    try {
      const { data } = await importItem({ variables: { input: {
        itemId: choosing.id,
        ...(listing.libraryItemId ? { libraryItemId: listing.libraryItemId } : { providerId: listing.providerId }),
        libraryId: listing.libraryId,
      } } });
      const result = assertSuccess(data?.importIdentifiedItem, "Could not import this");
      toast.success(`Imported ${result.matchCount} ${result.matchCount === 1 ? "file" : "files"}`);
      setChoosing(null);
      refresh();
    } catch (error) { toast.danger(errorMessage(error)); }
  };
  if (state.loading) return <div className="grid h-40 place-items-center"><Spinner size={28} /></div>;
  // Someone has to read why these couldn't be settled, and one result has nothing to compare it with.
  const opened = state.items.filter((item) => state.items.length === 1 || item.status === "needs_choice" || item.status === "unknown").map(getItemId);
  return (
    <div className="flex h-full min-h-0 min-w-0 flex-col">
      {state.items.length === 0 ? (
        <EmptyState compact icon={IconSearch} title="Nothing identified yet" description="This fills in as the download's files arrive." action={isAdmin ? <RunAgain infoHash={infoHash} onDone={refresh} /> : undefined} />
      ) : (
        <ResultsContext.Provider value={results}>
          <DataTable<Item>
            id="librarian.identified-items"
            label="Identified titles"
            queryKey={infoHash}
            columns={COLUMNS}
            rows={state.items}
            rowCount={state.items.length}
            getRowId={getItemId}
            getRowLabel={getItemLabel}
            selectable={false}
            actions={isAdmin ? actions : undefined}
            noun={NOUN}
            renderRowDetail={renderItemFindings}
            expanderColumn="details"
            expandAll={false}
            rowDetailHeight={11}
            defaultState={{ expanded: opened }}
            toolbar={isAdmin ? <div className="flex items-center gap-2"><TreatAs infoHash={infoHash} kind={kind} onDone={refresh} /><RunAgain infoHash={infoHash} onDone={refresh} /></div> : undefined}
          />
        </ResultsContext.Provider>
      )}
      <ConfirmDialog
        isOpen={upgrading !== null}
        onOpenChange={(open) => { if (!open) setUpgrading(null); }}
        title={`Use the better copies of ${upgrading?.title ?? upgrading?.label ?? "this"}?`}
        description={upgrading?.summary ?? undefined}
        confirmLabel="Use better copies"
        onConfirm={() => {
          if (upgrading) results.importIt(upgrading, { betterCopies: true });
          setUpgrading(null);
        }}
      >
        <p className="text-body-sm text-muted">The files your library has now stay on disk as second copies, so nothing is lost. You can remove them from the album&apos;s Files tab.</p>
      </ConfirmDialog>
      <ChooseListingDialog
        isOpen={choosing !== null}
        onOpenChange={(open) => { if (!open) setChoosing(null); }}
        title={`What is ${choosing?.label ?? "this"}?`}
        initialQuery={choosing?.title ?? choosing?.label ?? ""}
        listings={candidates.data?.identificationCandidates ?? []}
        loading={candidates.loading}
        error={candidates.error ? errorMessage(candidates.error) : null}
        search={(query) => { if (choosing) void search({ variables: { itemId: choosing.id, query } }); }}
        choose={choose}
        chooseLabel="Import as this"
      />
    </div>
  );
}

const STEP: Record<string, { tone: NonNullable<ActivityItem["tone"]>; icon: ActivityItem["icon"] }> = {
  running: { tone: "info", icon: <Spinner size={14} /> },
  found: { tone: "success", icon: <IconCheck /> },
  done: { tone: "neutral", icon: <IconCheck /> },
  warning: { tone: "warning", icon: <IconInfo /> },
  failed: { tone: "danger", icon: <IconInfo /> },
  skipped: { tone: "neutral", icon: <IconClock /> },
};

/**
 * Each run of identification, newest first. One run shows its steps and timings at a time, and
 * choosing a step shows what it found. Until someone chooses, the newest run stays open.
 */
export function IdentificationActivity({ state }: { state: TorrentIdentificationState }) {
  const [chosen, setChosen] = useState<string | null | undefined>(undefined);
  const runs = useMemo(() => {
    const byRun = new Map<string, Event[]>();
    for (const event of state.events) byRun.set(event.runId, [...(byRun.get(event.runId) ?? []), event]);
    return [...byRun.entries()].sort(([a], [b]) => b.localeCompare(a)).map(([id, steps]) => ({ id, steps: [...steps].sort((a, b) => a.sequence - b.sequence) }));
  }, [state.events]);
  const selected = chosen === undefined ? runs[0]?.id ?? null : chosen;
  const openRun = runs.find((run) => run.id === selected || run.steps.some((step) => step.id === selected))?.id;
  const items = useMemo<ActivityItem[]>(() => runs.map(({ id, steps }) => {
    const first = steps[0]!;
    const last = steps[steps.length - 1]!;
    const running = steps.some((step) => step.status === "running");
    const total = steps.reduce((sum, step) => sum + (step.durationMs ?? 0), 0);
    return {
      id,
      at: atTime(first.createdAt),
      tone: running ? "info" : "neutral",
      icon: running ? <Spinner size={14} /> : <IconClock />,
      content: <><strong>{first.trigger}</strong>{running ? " · Working" : ` · ${last.message} · ${seconds(total)}`}</>,
      children: id === openRun ? steps.map((step) => {
        const look = STEP[step.status] ?? STEP.done!;
        return {
          id: step.id,
          at: atTime(step.createdAt),
          tone: look.tone,
          icon: look.icon,
          content: <>{step.message}{step.durationMs != null ? <span className="text-muted"> · {seconds(step.durationMs)}</span> : null}</>,
          detail: step.detail && step.id === selected ? <p className="whitespace-pre-wrap text-label-sm [overflow-wrap:anywhere]">{step.detail}</p> : undefined,
        } satisfies ActivityItem;
      }) : undefined,
    } satisfies ActivityItem;
  }), [runs, openRun, selected]);
  if (state.loading) return <div className="grid h-40 place-items-center"><Spinner size={28} /></div>;
  return (
    <ScrollArea aria-label="Identification activity" className="h-full min-h-0">
      <ActivityFeed variant="timeline" items={items} empty="Nothing has been looked at yet." selectedId={selected} onSelectedIdChange={(id) => setChosen(id)} />
    </ScrollArea>
  );
}
