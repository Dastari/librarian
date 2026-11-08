import { useMutation } from "@apollo/client/react";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, IdentityCell, TextCell, RelativeTime } from "glass-ux/data-table";
import { Field } from "glass-ux/field";
import { IconEdit, IconPlus, IconTrash } from "glass-ux/icons";
import { Input } from "glass-ux/input";
import { SettingRow } from "glass-ux/setting-row";
import { Switch } from "glass-ux/switch";
import { toast } from "glass-ux/toast";
import { IconPower, IconRss } from "@tabler/icons-react";
import { useEffect, useState } from "react";

import { Button, Dialog, FieldGroup } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import {
  EntityRssFeedCreateDocument,
  EntityRssFeedDeleteDocument,
  EntityRssFeedListDocument,
  EntityRssFeedUpdateDocument,
  type RssFeedFieldsFragment,
} from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { glyph } from "@/lib/entities/kinds";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { runSettingsSelection } from "./runSettingsSelection";
import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

type Feed = RssFeedFieldsFragment;

/** A feed is never read more often than this; the server raises a shorter interval to it. */
export const MIN_FEED_MINUTES = 15;

const FeedGlyph = glyph(IconRss, "rss");
const PowerIcon = glyph(IconPower, "power");

/** Enabled and reading, enabled but failing, or switched off. */
export function feedState(feed: Pick<Feed, "enabled" | "lastError" | "consecutiveFailures">): { label: string; tone: "success" | "danger" | "neutral"; status: "online" | "error" | "offline" } {
  if (!feed.enabled) return { label: "Disabled", tone: "neutral", status: "offline" };
  return feed.lastError ? { label: `Failing ×${feed.consecutiveFailures ?? 1}`, tone: "danger", status: "error" } : { label: "Enabled", tone: "success", status: "online" };
}

const column = createDataTableColumns<Feed>();
const columns = column.columns([
  column.accessor("name", {
    header: "Feed", width: 26, priority: 1, cardRole: "title",
    cell: ({ row }) => { const state = feedState(row.original); return <IdentityCell icon={<FeedGlyph />} title={row.original.name} subtitle={row.original.lastError ?? hostOf(row.original.url)} status={state.status} statusLabel={state.label} />; },
  }),
  column.display({ id: "status", header: "Status", width: 9, priority: 2, cardRole: "status", cell: ({ row }) => { const state = feedState(row.original); return <Badge size="sm" marker="dot" tone={state.tone}>{state.label}</Badge>; } }),
  column.accessor("pollIntervalMinutes", { header: "Reads", width: 9, priority: 3, cell: ({ row }) => <TextCell muted>{`every ${Math.max(row.original.pollIntervalMinutes, MIN_FEED_MINUTES)} min`}</TextCell> }),
  column.accessor((feed) => feed.lastSuccessfulAt ?? "", { id: "last", header: "Last read", width: 9, priority: 4, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastSuccessfulAt)} fallback="Never" /> }),
]);

/** The host of a feed address, so the address itself (which may carry a key) is not shown in the table. */
function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return "Invalid address";
  }
}

/**
 * RSS feeds the server reads for new releases of what is wanted. Only administrators can add or
 * change one, because the server fetches the address.
 */
export function RssFeeds() {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const list = useSettingsConnection(EntityRssFeedListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.rssFeeds);
  const { rows, loading, refetch } = list;
  const [editing, setEditing] = useState<Feed | null | "new">(null);
  const [update] = useMutation(EntityRssFeedUpdateDocument);
  const [remove] = useMutation(EntityRssFeedDeleteDocument);

  const toggle = async (feed: Feed) => {
    try {
      assertSuccess((await update({ variables: { id: feed.id, input: { enabled: !feed.enabled } } })).data?.updateRssFeed, "Could not update the feed");
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const actions = defineActions<Feed>("rss-feed", isAdmin ? [
    { id: "edit", bulk: true, label: "Edit", icon: IconEdit, group: "open", priority: 2, visible: (feeds) => feeds.length === 1, run: (feeds) => setEditing(feeds[0]!) },
    { id: "toggle", bulk: true, label: (feeds) => (feeds[0]!.enabled ? "Disable" : "Enable"), icon: PowerIcon, group: "manage", priority: 2, visible: (feeds) => feeds.length === 1, run: (feeds) => toggle(feeds[0]!) },
    {
      id: "delete", label: (feeds) => feeds.length === 1 ? "Remove" : `Remove ${feeds.length} feeds`, icon: IconTrash, group: "danger", bulk: true,
      confirm: (feeds) => ({ title: feeds.length === 1 ? `Remove ${feeds[0]!.name}?` : `Remove ${feeds.length} feeds?`, body: "The feed is no longer read. Downloads already started keep running.", confirmLabel: "Remove", tone: "danger" }),
      run: (feeds) => runSettingsSelection(feeds, "Remove", (feed) => feed.name, async (feed) => assertSuccess((await remove({ variables: { id: feed.id } })).data?.deleteRssFeed, "Could not remove the feed"), refetch),
      successMessage: (feeds) => feeds.length === 1 ? "Feed removed" : `${feeds.length} feeds removed`,
    },
  ] : []);

  return (
    <>
      <SettingsSection
        title="RSS feeds"
        description="Read on their own schedule for new releases of the shows and movies you want. The cheapest way to find a new episode; the air-time search is the backup."
        actions={isAdmin ? <Button variant="primary" size="sm" onPress={() => setEditing("new")}><IconPlus /> Add feed</Button> : undefined}
      >
        <SettingsLoadError error={list.error} retry={list.refetch} hasRows={rows.length > 0} />
        <LoadedDataTable<Feed> complete={!list.loading && !list.error} height="auto" selectable={isAdmin} queryKey={user?.id}
          id={`librarian.${user?.id ?? "session"}.settings.rss-feeds`}
          label="RSS feeds"
          columns={columns}
          rows={rows}
          getRowId={(feed) => feed.id}
          getRowLabel={(feed) => feed.name}
          loading={loading && rows.length === 0}
          actions={actions} renderBulkActions={(selected) => <TableBulkActions set={actions} rows={selected} noun={{ one: "feed", other: "feeds" }} />}
          onRowOpen={isAdmin ? (feed) => setEditing(feed) : undefined}
          rowOpen="click"
          noun={{ one: "feed", other: "feeds" }}
          emptyMessage={isAdmin ? "No feeds yet. Add one to find new releases without searching." : "No feeds."}
        />
      </SettingsSection>
      <FeedDialog feed={editing === "new" ? null : editing} isOpen={editing !== null} onOpenChange={(open) => { if (!open) setEditing(null); }} onSaved={() => void refetch()} />
    </>
  );
}

interface FeedDialogProps {
  feed: Feed | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: () => void;
}

/** Add or edit a feed: its name, address, how often to read it and whether it is on. */
export function FeedDialog({ feed, isOpen, onOpenChange, onSaved }: FeedDialogProps) {
  const { user } = useSession();
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [minutes, setMinutes] = useState("30");
  const [enabled, setEnabled] = useState(true);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [create, { loading: creating }] = useMutation(EntityRssFeedCreateDocument);
  const [update, { loading: updating }] = useMutation(EntityRssFeedUpdateDocument);

  useEffect(() => {
    if (!isOpen) return;
    setName(feed?.name ?? "");
    setUrl(feed?.url ?? "");
    setMinutes(String(feed?.pollIntervalMinutes ?? 30));
    setEnabled(feed?.enabled ?? true);
    setErrors({});
  }, [isOpen, feed]);

  const save = async () => {
    const next: Record<string, string> = {};
    const interval = Number(minutes);
    if (!name.trim()) next.name = "Give the feed a name";
    if (!/^https?:\/\/\S+$/i.test(url.trim())) next.url = "Enter an http or https address";
    if (!Number.isInteger(interval) || interval < MIN_FEED_MINUTES) next.minutes = `At least ${MIN_FEED_MINUTES} minutes`;
    setErrors(next);
    if (Object.keys(next).length) return;
    const common = { name: name.trim(), url: url.trim(), pollIntervalMinutes: interval, enabled };
    try {
      if (feed) {
        assertSuccess((await update({ variables: { id: feed.id, input: common } })).data?.updateRssFeed, "Could not save the feed");
        toast.success("Feed saved");
      } else {
        if (!user) throw new Error("Sign in to add a feed");
        assertSuccess((await create({ variables: { input: { ...common, userId: user.id } } })).data?.createRssFeed, "Could not add the feed");
        toast.success("Feed added");
      }
      onSaved();
      onOpenChange(false);
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={feed ? `Edit ${feed.name}` : "Add feed"}
      size="md"
      footer={
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" onPress={() => void save()} isPending={creating || updating}>{feed ? "Save" : "Add"}</Button>
        </>
      }
    >
      <FieldGroup>
        <Field label="Name" error={errors.name} required>
          <Input value={name} onValueChange={setName} required />
        </Field>
        <Field label="Feed address" error={errors.url} hint="An RSS 2.0 feed of releases. Torznab feeds work too." required>
          <Input value={url} onValueChange={setUrl} type="url" className="font-mono" required />
        </Field>
        <Field label="Read every (minutes)" error={errors.minutes} hint={`Never faster than every ${MIN_FEED_MINUTES} minutes, and slower while the feed keeps failing.`}>
          <Input value={minutes} onValueChange={setMinutes} inputMode="numeric" />
        </Field>
        <SettingRow label="Enabled" description="A disabled feed is not read." control={<Switch checked={enabled} onCheckedChange={setEnabled} />} />
      </FieldGroup>
    </Dialog>
  );
}
