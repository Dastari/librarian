import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { byteSizeSchema } from "glass-ux/byte-size-field";
import { durationSchema } from "glass-ux/duration-field";
import { Form, FormActions, numberRule, useAppForm } from "glass-ux/form";
import { IconFolder, IconPlay } from "glass-ux/icons";
import { SettingRow } from "glass-ux/setting-row";
import { useEffect, useRef, useState } from "react";
import { z } from "zod";

import { Button } from "@/components/ui";
import { MODE_HELP } from "@/features/acquisition/mode";
import { FreeSpaceCell } from "@/features/libraries/FreeSpaceCell";
import { spaceThresholds } from "@/features/libraries/volumeSpace";
import { EntityStorageVolumeListDocument, TriggerAutoDownloadDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

import { AutoDownloadStatus } from "./AutoDownloadStatus";
import { BlockedReleases } from "./BlockedReleases";
import { MoveDownloadsDialog } from "./MoveDownloadsDialog";
import { ByteSizeRow, DurationRow, NumberRow, TextRow, settingsFormBehaviour } from "./SettingFields";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { asBool, asNumber, useAppSettings } from "./useAppSettings";
import { useSettingsConnection } from "./useSettingsConnection";

/** Intervals are kept as minutes on the server and edited as seconds, the way `DurationRow` holds them. */
export const minutesToSeconds = (minutes: number) => Math.round(minutes * 60);
export const secondsToMinutes = (seconds: number) => Math.round(seconds / 60);

const count = (min: number, max: number | undefined, noun: string) => numberRule({
  required: true, integer: true, min, max,
  requiredMessage: "Enter a number",
  rangeMessage: max === undefined ? `Enter a whole number of at least ${min}` : `Enter a whole number from ${min} to ${max}${noun}`,
});
const minutes = (min: number, max: number) => durationSchema({ required: true, min: minutesToSeconds(min), max: minutesToSeconds(max) });

const schema = z.object({
  sessionDir: z.string().trim().min(1, "Choose a session folder"),
  listenPort: count(1024, 65535, ""),
  enableDht: z.boolean(),
  maxConcurrent: count(1, 50, ""),
  // Bytes per second, as the client reads them; 0 is unlimited.
  downloadLimit: byteSizeSchema({ required: true, min: 0 }),
  uploadLimit: byteSizeSchema({ required: true, min: 0 }),
  autoEnabled: z.boolean(),
  autoInterval: minutes(5, 1440),
  autoBackoff: minutes(5, 10080),
  airGrace: minutes(0, 1440),
  airWindow: count(0, 60, " days"),
  airRetry: minutes(5, 1440),
  maxGrabs: count(1, 100, ""),
  hourlySearchBudget: count(0, 600, ""),
  onlyMissingFiles: z.boolean(),
  seedRatioLimit: numberRule({ required: true, min: 0, max: 100, requiredMessage: "Enter a number", rangeMessage: "Enter a ratio from 0 to 100" }),
  seedTime: minutes(0, 100000),
  removeAfterImport: z.boolean(),
});
type Values = z.input<typeof schema>;

export function DownloadsSettings() {
  const torrent = useAppSettings("torrent");
  const auto = useAppSettings("auto_download");
  const [trigger, { loading: triggering }] = useMutation(TriggerAutoDownloadDocument);
  const [movingDownloads, setMovingDownloads] = useState(false);
  const downloadDir = torrent.values.get("torrent.download_dir") ?? "";
  // The download folder's disk or share and its free space (design.md Q146).
  const downloadVolume = useSettingsConnection(EntityStorageVolumeListDocument, { where: { holdsDownloads: { eq: true } } }, (data) => data.storageVolumes);
  const storage = useAppSettings("storage");
  // Intervals are minutes in the settings and seconds in the form (see `minutesToSeconds`).
  const defaults = useRef<Values>({ sessionDir: "", listenPort: 6881, enableDht: true, maxConcurrent: 5, downloadLimit: 0, uploadLimit: 0, autoEnabled: false, autoInterval: minutesToSeconds(60), autoBackoff: minutesToSeconds(360), airGrace: minutesToSeconds(60), airWindow: 7, airRetry: minutesToSeconds(60), maxGrabs: 5, hourlySearchBudget: 30, onlyMissingFiles: true, seedRatioLimit: 1, seedTime: 0, removeAfterImport: false });
  const form = useAppForm({ ...settingsFormBehaviour, validators: { onDynamic: schema }, defaultValues: defaults.current, onSubmit: async ({ value }) => {
    const values = schema.parse(value) as Values;
    // Only what was edited is written, so saving one group does not create rows for every
    // default on the page.
    const baseline = defaults.current;
    const edited = (fields: Array<[keyof Values, string, ((value: number) => number)?]>) => Object.fromEntries(fields.filter(([field]) => !Object.is(values[field], baseline[field])).map(([field, key, toStored]) => [key, toStored ? toStored(values[field] as number) : values[field]]));
    await torrent.save(edited([["sessionDir", "torrent.session_dir"], ["listenPort", "torrent.listen_port"], ["enableDht", "torrent.enable_dht"], ["maxConcurrent", "torrent.max_concurrent"], ["downloadLimit", "torrent.download_limit"], ["uploadLimit", "torrent.upload_limit"], ["seedRatioLimit", "torrent.seed_ratio_limit"], ["seedTime", "torrent.seed_time_minutes", secondsToMinutes], ["removeAfterImport", "torrent.remove_after_import"]]));
    await auto.save(edited([["autoEnabled", "auto_download.enabled"], ["autoInterval", "auto_download.interval_minutes", secondsToMinutes], ["autoBackoff", "auto_download.search_backoff_minutes", secondsToMinutes], ["airGrace", "auto_download.air_grace_minutes", secondsToMinutes], ["airWindow", "auto_download.air_window_days"], ["airRetry", "auto_download.air_retry_minutes", secondsToMinutes], ["maxGrabs", "auto_download.max_concurrent_grabs"], ["hourlySearchBudget", "auto_download.hourly_search_budget"], ["onlyMissingFiles", "auto_download.only_missing_files"]]));
    defaults.current = values;
    toast.success("Settings saved");
  } });

  useEffect(() => {
    const t = torrent.values;
    const a = auto.values;
    // Initial category responses and live snapshots can arrive independently.
    // Update untouched fields/defaults without discarding an in-progress edit.
    const snapshot: Values = {
      sessionDir: t.get("torrent.session_dir") ?? "",
      listenPort: asNumber(t.get("torrent.listen_port"), 6881),
      enableDht: asBool(t.get("torrent.enable_dht"), true),
      maxConcurrent: asNumber(t.get("torrent.max_concurrent"), 5),
      downloadLimit: asNumber(t.get("torrent.download_limit"), 0),
      uploadLimit: asNumber(t.get("torrent.upload_limit"), 0),
      seedRatioLimit: asNumber(t.get("torrent.seed_ratio_limit"), 1),
      seedTime: minutesToSeconds(asNumber(t.get("torrent.seed_time_minutes"), 0)),
      removeAfterImport: asBool(t.get("torrent.remove_after_import"), false),
      // Off until someone turns it on, as the server has it (download_monitor.rs).
      autoEnabled: asBool(a.get("auto_download.enabled"), false),
      autoInterval: minutesToSeconds(asNumber(a.get("auto_download.interval_minutes"), 60)),
      autoBackoff: minutesToSeconds(asNumber(a.get("auto_download.search_backoff_minutes"), 360)),
      airGrace: minutesToSeconds(asNumber(a.get("auto_download.air_grace_minutes"), 60)),
      airWindow: asNumber(a.get("auto_download.air_window_days"), 7),
      airRetry: minutesToSeconds(asNumber(a.get("auto_download.air_retry_minutes"), 60)),
      maxGrabs: asNumber(a.get("auto_download.max_concurrent_grabs"), 5),
      hourlySearchBudget: asNumber(a.get("auto_download.hourly_search_budget"), 30),
      onlyMissingFiles: asBool(a.get("auto_download.only_missing_files"), true),
    };
    const current = form.state.values;
    const baseline = defaults.current;
    const edited = (Object.keys(snapshot) as Array<keyof Values>).filter((key) => !Object.is(current[key], baseline[key]));
    defaults.current = snapshot;
    form.reset(snapshot);
    for (const key of edited) form.setFieldValue(key, current[key]);
  }, [torrent.values, auto.values, form]);

  const runNow = async () => {
    try {
      const { data } = await trigger({ variables: { libraryId: null } });
      const result = data?.triggerAutoDownload;
      if (result?.success) toast.success(`Considered ${result.candidatesConsidered}, searched ${result.searched}, grabbed ${result.grabbed}`);
      else toast.warning(result?.error ?? result?.errors.join(", ") ?? "Auto-download did not run");
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <SettingsPage>
      <Form form={form} className="settings-measure flex min-w-0 flex-col gap-8" onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))}>
        <SettingsSection
          title="Automatic downloads"
          width="readable"
          description="Librarian watches for what you want and sends the best match to the download client, on a schedule and from your RSS feeds. Feeds and sources are set up under Sources."
        >
          <form.AppField name="autoEnabled">{(field) => <field.SwitchRow label="Search and download automatically" description="Off by default, and shows what the server does. Turned on, Librarian searches every wanted title once an hour and also searches each new episode, and each wanted movie out on digital or disc, shortly after it comes out: your feeds first, then your indexers within the hourly search limit, retrying less often over the following days. It downloads the best release your quality profile accepts. While off, nothing searches or downloads on its own (feeds are still read into their catalogue, and recommendations are still looked up); Search now and Run now still work." />}</form.AppField>
          <SettingRow
            label="Recent activity"
            description={<AutoDownloadStatus settings={auto.values} />}
            control={<Button size="sm" variant="secondary" onPress={() => void runNow()} isPending={triggering}><IconPlay size="md" /> Run now</Button>}
          />
          <form.AppField name="autoInterval">{() => <DurationRow label="Check everything every" description="A full sweep of every wanted title." units={["d", "h", "m"]} defaultUnit="m" />}</form.AppField>
          <form.AppField name="autoBackoff">{() => <DurationRow label="Search a missing item again after" description="Keeps one missing item from being searched on every sweep." units={["d", "h", "m"]} defaultUnit="m" />}</form.AppField>
          <form.AppField name="airGrace">{() => <DurationRow label="Wait after an episode airs" description="Releases take a while to appear. Nothing is searched for before this has passed." units={["h", "m"]} defaultUnit="m" />}</form.AppField>
          <form.AppField name="airWindow">{() => <NumberRow label="Watch new episodes for" description="How long a new episode is searched for on the quicker schedule. 0 skips it." min={0} max={60} unit="days" />}</form.AppField>
          <form.AppField name="airRetry">{() => <DurationRow label="Search a new episode again after" description="The first retries after it airs. The wait then grows with the time since it aired, up to the ordinary wait above, until the days above run out." units={["h", "m"]} defaultUnit="m" />}</form.AppField>
          <form.AppField name="maxGrabs">{() => <NumberRow label="Most automatic downloads at once" description="Anything more waits for a free slot. Searching by hand is not limited." min={1} max={100} />}</form.AppField>
          <form.AppField name="hourlySearchBudget">{() => <NumberRow label="Indexer searches for recommendations and new releases" description="Looking up the best release for what is missing, and searching for new episodes and movies after they come out, never exceed this together, so indexers are not hammered. Feeds are used first and cost nothing. 0 turns recommendations and those searches off." min={0} max={600} step={5} unit="per hour" />}</form.AppField>
          <form.AppField name="onlyMissingFiles">{(field) => <field.SwitchRow label="Download only the missing files from packs" description="A season pack or album downloads just the episodes or tracks you are missing, not the whole thing. On a private tracker this can cost ratio: a partial download never completes the torrent, so it cannot be seeded to 100%, and some trackers count an unfinished torrent against your account. Turn it off to always take the whole pack." />}</form.AppField>
          <AutoDownloadGuide />
        </SettingsSection>

        <SettingsSection title="Torrent client" width="readable" description="The client reads these settings when it starts, so every change on this page, including seeding, takes effect after the server restarts. Moving the download folder restarts the client itself.">
          <SettingRow
            label="Download folder"
            description={<><span className="font-mono text-foreground [overflow-wrap:anywhere]">{downloadDir || "/data/downloads"}</span><br />A folder on this server or on a saved network share. Moving it checks a copy you made first, then points every torrent at it.</>}
            control={<Button size="sm" variant="secondary" onPress={() => setMovingDownloads(true)}><IconFolder size="md" /> Move…</Button>}
          />
          <SettingRow
            label="Free space"
            description={downloadVolume.rows[0]?.label ? `On ${downloadVolume.rows[0].label}. Read every few minutes; when it is critically low, releases that would not fit are not downloaded.` : "Read every few minutes; when it is critically low, releases that would not fit are not downloaded."}
            control={<div className="w-56 max-w-full"><FreeSpaceCell volume={downloadVolume.rows[0] ?? null} thresholds={spaceThresholds(storage.values)} /></div>}
          />
          <form.AppField name="sessionDir">{() => <TextRow mono label="Session folder" description="Resume data and DHT state." />}</form.AppField>
          <form.AppField name="listenPort">{() => <NumberRow label="Listen port" min={1024} max={65535} format={{ useGrouping: false }} />}</form.AppField>
          <form.AppField name="maxConcurrent">{() => <NumberRow label="Concurrent downloads" min={1} max={50} />}</form.AppField>
          <form.AppField name="downloadLimit">{() => <ByteSizeRow label="Download speed limit" description="Per second. 0 is unlimited." defaultUnit="MB" />}</form.AppField>
          <form.AppField name="uploadLimit">{() => <ByteSizeRow label="Upload speed limit" description="Per second. 0 is unlimited." defaultUnit="MB" />}</form.AppField>
          <form.AppField name="enableDht">{(field) => <field.SwitchRow label="Enable DHT" description="Find peers without a tracker. Turn off for private-tracker-only setups." />}</form.AppField>
        </SettingsSection>

        <SettingsSection title="Seeding" width="readable" description="When a finished download stops sharing.">
          <form.AppField name="seedRatioLimit">{() => <NumberRow label="Seed until ratio" description="0 = no ratio rule." min={0} max={100} step={0.1} />}</form.AppField>
          <form.AppField name="seedTime">{() => <DurationRow label="Seed for" description="0 = no time rule." units={["d", "h", "m"]} defaultUnit="m" />}</form.AppField>
          <form.AppField name="removeAfterImport">{(field) => <field.SwitchRow label="Remove the torrent and its files after import" description="Off keeps the files and only stops seeding." />}</form.AppField>
        </SettingsSection>

        <FormActions>
          <form.Subscribe selector={(state) => state.isSubmitting}>{(pending) => <form.CancelButton disabled={pending}>Discard</form.CancelButton>}</form.Subscribe>
          <form.SubmitButton>Save</form.SubmitButton>
        </FormActions>
      </Form>
      <BlockedReleases />
      <MoveDownloadsDialog open={movingDownloads} onOpenChange={setMovingDownloads} current={downloadDir} />
    </SettingsPage>
  );
}

/** What each choice on a show, album, audiobook or movie does once automatic downloading is on. */
function AutoDownloadGuide() {
  const rows: Array<[string, string]> = [
    ["Shows, albums and audiobooks", `Set per title.  Off: ${MODE_HELP.NONE}  Wanted only: ${MODE_HELP.WANTED}  Everything missing: ${MODE_HELP.ALL}`],
    ["Movies", "A movie is downloaded when it is monitored and wanted. When its digital or disc release date is known it is searched from that day, like a new episode; otherwise once its release date has passed."],
    ["New episodes", "Searched after they air plus the wait above: your feeds first, then your indexers within the hourly search limit, then more and more rarely for the days above, then on the ordinary schedule. Nothing tells you about a new episode that has not turned up until a day has passed, and a notice you dismiss is not raised again."],
    ["Always", "Only the best release your quality profile accepts is taken; blocklisted releases are never taken again. Each search, grab and reason for skipping appears under Activity."],
  ];
  return (
    <dl className="mt-2 flex flex-col gap-2 px-1 text-label-sm text-muted" aria-label="What automatic downloads do">
      {rows.map(([term, text]) => (
        <div key={term}>
          <dt className="font-medium text-foreground">{term}</dt>
          <dd>{text}</dd>
        </div>
      ))}
    </dl>
  );
}
