import { useMutation, useQuery } from "@apollo/client/react";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, IdentityCell, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconArrowUpCircle, IconHistory } from "@tabler/icons-react";
import { IconExternalLink, IconRefresh } from "glass-ux/icons";
import { Notice } from "glass-ux/notice";
import { RichTextReader } from "glass-ux/rich-text-reader";
import { Segmented } from "glass-ux/segmented";
import { SettingRow } from "glass-ux/setting-row";
import { Switch } from "glass-ux/switch";
import { toast } from "glass-ux/toast";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { Button, ConfirmDialog } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { CheckForServerUpdateDocument, ServerUpdateStatusDocument, UpgradeServerDocument } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { glyph } from "@/lib/entities/kinds";
import { formatBytes, parseTimestamp } from "@/lib/format";
import { errorMessage } from "@/lib/graphql/errors";

import { buildKindLabel, outcomeLabel, stageLabel, summary, upgradeRunning, type UpgradeOutcome } from "./serverUpdate";
import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { useAppSettings } from "./useAppSettings";

const UpgradeIcon = glyph(IconArrowUpCircle, "arrow-up-circle");
const HistoryGlyph = glyph(IconHistory, "history");

const column = createDataTableColumns<UpgradeOutcome>();
const columns = column.columns([
  column.accessor((row) => `${row.fromVersion} → ${row.toVersion}`, {
    id: "versions", header: "Upgrade", width: 16, priority: 1, cardRole: "title",
    cell: ({ row }) => <IdentityCell icon={<HistoryGlyph />} title={`${row.original.fromVersion} → ${row.original.toVersion}`} subtitle={row.original.detail ?? undefined} />,
  }),
  column.accessor("outcome", {
    header: "Outcome", width: 8, priority: 2,
    cell: ({ row }) => { const outcome = outcomeLabel(row.original.outcome); return <Badge size="sm" marker="dot" tone={outcome.tone}>{outcome.label}</Badge>; },
  }),
  column.accessor("by", { header: "By", width: 8, priority: 3, cell: ({ row }) => <TextCell muted>{row.original.by}</TextCell> }),
  column.accessor("at", { header: "When", width: 9, priority: 2, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.at)} /> }),
]);

/** A read-only fact: name left, value right. */
function Fact({ label, value, description, mono }: { label: string; value: ReactNode; description?: ReactNode; mono?: boolean }) {
  return <SettingRow label={label} description={description} control={<span className={mono ? "text-body-sm break-all font-mono text-muted" : "text-body-sm text-muted"}>{value}</span>} />;
}

function ExternalLink({ href, children }: { href: string; children: ReactNode }) {
  return <a className="nav-focus inline-flex items-center gap-1 rounded text-brand hover:underline" href={href} target="_blank" rel="noreferrer noopener">{children}<IconExternalLink size="sm" aria-hidden /></a>;
}

/** When, as words, for a fact row. */
function When({ value }: { value: string | null | undefined }) {
  const date = parseTimestamp(value);
  return date ? <RelativeTime value={date} /> : <>Never</>;
}

/** Settings › Updates: what runs here, the newest release on the channel, and the upgrade (design.md Q151–Q152). */
export function UpdatesSettings() {
  const { user } = useSession();
  const [polling, setPolling] = useState(false);
  const { data, previousData, loading, error, refetch } = useQuery(ServerUpdateStatusDocument, { pollInterval: polling ? 2000 : 0, notifyOnNetworkStatusChange: false });
  const status = (data ?? previousData)?.serverUpdateStatus;
  const settings = useAppSettings("updates");
  const [check, { loading: checking }] = useMutation(CheckForServerUpdateDocument);
  const [upgrade] = useMutation(UpgradeServerDocument);
  const [confirming, setConfirming] = useState(false);
  // The version an upgrade started from, so the page can say when the restart has finished.
  const upgradingFrom = useRef<string | null>(null);

  const running = upgradeRunning(status);
  const restarting = status?.progress?.stage === "restarting";
  useEffect(() => {
    setPolling(running || upgradingFrom.current !== null);
  }, [running]);
  useEffect(() => {
    // Progress lives in the server's memory, so a status without it after an upgrade started is
    // a server that has restarted: into the new version, or back into this one.
    const from = upgradingFrom.current;
    if (!from || !data?.serverUpdateStatus || data.serverUpdateStatus.progress) return;
    const now = data.serverUpdateStatus;
    upgradingFrom.current = null;
    setPolling(false);
    if (now.currentVersion !== from) toast.success(`Librarian ${now.currentVersion} is running`);
    else toast.danger(now.history[0]?.detail ?? `The upgrade did not take; ${from} is still running`);
  }, [data]);

  const save = async (entries: Record<string, string | boolean>) => {
    try {
      await settings.save(entries);
      await refetch();
    } catch (failure) {
      toast.danger(errorMessage(failure, "Could not save the setting"));
    }
  };

  const checkNow = async () => {
    try {
      const result = (await check()).data?.checkForServerUpdate;
      if (result?.error) toast.danger(result.error);
      else toast.success(result?.status ? summary(result.status) : "Checked");
      await refetch();
    } catch (failure) {
      toast.danger(errorMessage(failure, "Could not check for updates"));
    }
  };

  const latest = status?.latest;
  const startUpgrade = async () => {
    if (!latest || !status) return;
    const result = (await upgrade({ variables: { version: latest.version } })).data?.upgradeServer;
    if (!result?.success) throw new Error(result?.error ?? "The upgrade did not start");
    upgradingFrom.current = status.currentVersion;
    setPolling(true);
    setConfirming(false);
    toast.info(`Upgrading to ${latest.version}`);
    await refetch();
  };

  // While the server restarts the query fails; that is expected, not an error to show.
  const reconnecting = Boolean(error) && upgradingFrom.current !== null;

  return (
    <SettingsPage>
      <SettingsLoadError error={reconnecting ? undefined : error} retry={() => refetch().catch(() => {})} hasRows={Boolean(status)} />
      <SettingsSection title="This server" width="readable">
        <Fact label="Version" value={status ? status.currentVersion : loading ? "…" : "Unknown"} mono />
        <Fact label="Build" value={status ? buildKindLabel(status.buildKind) : "…"} description={status ? status.platform : undefined} />
        <Fact label="Releases from" value={status ? <ExternalLink href={`https://github.com/${status.repository}/releases`}>{status.repository}</ExternalLink> : "…"} />
      </SettingsSection>

      <SettingsSection
        title="Checking"
        width="readable"
        actions={<Button size="sm" variant="secondary" isPending={checking} isDisabled={!status || running} onPress={() => void checkNow()}><IconRefresh size="md" /> Check now</Button>}
      >
        <SettingRow
          label="Channel"
          description={status?.channelIsDefault
            ? `Stable has plain releases only; Dev adds pre-releases. Not chosen yet, so it follows this server's version (${status.channel === "dev" ? "a pre-release" : "a release"}).`
            : "Stable has plain releases only; Dev adds pre-releases."}
          control={<Segmented
            aria-label="Update channel"
            size="sm"
            collapse="none"
            options={[{ value: "stable", label: "Stable" }, { value: "dev", label: "Dev" }]}
            value={status?.channel === "dev" ? "dev" : "stable"}
            disabled={!status || running}
            onValueChange={(value) => void save({ "updates.channel": value })}
          />}
        />
        <SettingRow
          label="Check automatically"
          description="Ask GitHub every 6 hours and tell admins once about each new version."
          control={<Switch aria-label="Check automatically" checked={status?.autoCheck ?? true} disabled={!status} onCheckedChange={(checked) => void save({ "updates.auto_check": checked })} />}
        />
        <Fact
          label="Last checked"
          value={<When value={status?.lastCheckedAt} />}
          description={status?.lastError ? <span className="text-danger">{status.lastError}</span> : status ? summary(status) : undefined}
        />
        {status?.nextCheckAt ? <Fact label="Next check" value={<When value={status.nextCheckAt} />} /> : null}
      </SettingsSection>

      {status ? (
        <SettingsSection
          title={latest ? `Latest on ${status.channel === "dev" ? "Dev" : "Stable"}` : "Latest release"}
          width="readable"
          actions={status.canSelfUpgrade && latest ? (
            <Button size="sm" variant="primary" onPress={() => setConfirming(true)}><UpgradeIcon /> Upgrade to {latest.version}</Button>
          ) : undefined}
        >
          {latest ? <>
            <Fact
              label={latest.name && latest.name !== latest.tag ? `${latest.version} · ${latest.name}` : latest.version}
              value={<ExternalLink href={latest.url}>Release page</ExternalLink>}
              description={[latest.publishedAt ? `Published ${parseTimestamp(latest.publishedAt)?.toLocaleDateString() ?? ""}` : null, latest.prerelease ? "pre-release" : null, latest.assetName ? `${latest.assetName}${latest.assetSize ? `, ${formatBytes(latest.assetSize)}` : ""}` : null, latest.hasChecksums ? "SHA256SUMS published" : "no SHA256SUMS"].filter(Boolean).join(" · ")}
            />
            {status.progress ? (
              <Notice tone={status.progress.failed ? "danger" : "info"} title={`${stageLabel(status.progress.stage)} ${status.progress.targetVersion}`}>
                {status.progress.message}
                {restarting || reconnecting ? " Waiting for the server to come back…" : null}
              </Notice>
            ) : null}
            {status.updateAvailable && !status.canSelfUpgrade && status.selfUpgradeBlockedReason && !running ? (
              <Notice tone="warning" title="This server can't upgrade itself">
                {status.selfUpgradeBlockedReason}{status.selfUpgradeInstructions ? ` ${status.selfUpgradeInstructions}` : null}
              </Notice>
            ) : null}
            {latest.notes ? (
              <div className="rounded-[var(--radius-md)] px-1">
                {/* Release notes are Markdown from GitHub: the reader ignores raw HTML, sanitises links and loads no images. */}
                <RichTextReader value={latest.notes} images={false} anchors={false} maxHeight={420} aria-label={`Release notes for ${latest.version}`} />
              </div>
            ) : null}
          </> : (
            <p className="text-label-sm text-muted">{status.lastCheckedAt ? "GitHub lists no release on this channel." : "Press Check now to ask GitHub."}</p>
          )}
        </SettingsSection>
      ) : null}

      {status?.tvApp ? (
        <SettingsSection title="Android TV app" width="readable">
          <Fact label={`Latest: ${status.tvApp.version}`} value={<ExternalLink href={status.tvApp.apkUrl ?? status.tvApp.url}>{status.tvApp.apkUrl ? "Download APK" : "Release page"}</ExternalLink>} description="TV apps can read this to offer their own update." />
        </SettingsSection>
      ) : null}

      {status && status.history.length > 0 ? (
        <SettingsSection title="Upgrades" description="Each upgrade takes and verifies a full backup first. The binary it replaced stays beside the new one as .previous.">
          <LoadedDataTable<UpgradeOutcome>
            height="auto"
            queryKey={user?.id}
            id={`librarian.${user?.id ?? "session"}.settings.upgrades`}
            label="Upgrades"
            columns={columns}
            rows={status.history}
            getRowId={(row) => `${row.at}-${row.toVersion}-${row.outcome}`}
            getRowLabel={(row) => `${row.fromVersion} to ${row.toVersion}`}
            noun={{ one: "upgrade", other: "upgrades" }}
            emptyMessage="No upgrades yet."
          />
        </SettingsSection>
      ) : null}

      {latest && status ? (
        <ConfirmDialog
          isOpen={confirming}
          onOpenChange={setConfirming}
          title={`Upgrade to Librarian ${latest.version}?`}
          description={`Librarian downloads ${latest.assetName ?? "the release"} from GitHub, checks it against SHA256SUMS, takes and verifies a full backup, swaps the binary and restarts. Playback and downloads stop for the restart. If ${latest.version} doesn't start, ${status.currentVersion} is put back by itself.`}
          confirmLabel={`Upgrade to ${latest.version}`}
          onConfirm={startUpgrade}
        />
      ) : null}
    </SettingsPage>
  );
}
