import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { IconPlay } from "glass-ux/icons";
import { useEffect, useRef } from "react";
import { z } from "zod";

import { Button, FieldGroup, Panel } from "@/components/ui";
import { MODE_HELP } from "@/features/acquisition/mode";
import { TriggerAutoDownloadDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

import { AutoDownloadStatus } from "./AutoDownloadStatus";
import { BlockedReleases } from "./BlockedReleases";
import { asBool, asNumber, useAppSettings } from "./useAppSettings";

const requiredNumber = (rule: z.ZodNumber) => z.number().nullable().transform((value, context) => {
  if (value === null) {
    context.addIssue({ code: "custom", message: "Enter a number" });
    return z.NEVER;
  }
  return value;
}).pipe(rule);

const schema = z.object({
  downloadDir: z.string().trim().min(1, "Choose a download folder"),
  sessionDir: z.string().trim().min(1, "Choose a session folder"),
  listenPort: requiredNumber(z.number().int().min(1024).max(65535)),
  enableDht: z.boolean(),
  maxConcurrent: requiredNumber(z.number().int().min(1).max(50)),
  downloadLimit: requiredNumber(z.number().int().min(0)),
  uploadLimit: requiredNumber(z.number().int().min(0)),
  autoEnabled: z.boolean(),
  autoInterval: requiredNumber(z.number().int().min(5).max(1440)),
  autoBackoff: requiredNumber(z.number().int().min(5).max(10080)),
  airGrace: requiredNumber(z.number().int().min(0).max(1440)),
  airWindow: requiredNumber(z.number().int().min(0).max(60)),
  airRetry: requiredNumber(z.number().int().min(5).max(1440)),
  maxGrabs: requiredNumber(z.number().int().min(1).max(100)),
  seedRatioLimit: requiredNumber(z.number().min(0).max(100)),
  seedTimeMinutes: requiredNumber(z.number().int().min(0).max(100000)),
  removeAfterImport: z.boolean(),
});
type Values = z.input<typeof schema>;

export function DownloadsSettings() {
  const torrent = useAppSettings("torrent");
  const auto = useAppSettings("auto_download");
  const [trigger, { loading: triggering }] = useMutation(TriggerAutoDownloadDocument);
  const defaults = useRef<Values>({ downloadDir: "", sessionDir: "", listenPort: 6881, enableDht: true, maxConcurrent: 5, downloadLimit: 0, uploadLimit: 0, autoEnabled: false, autoInterval: 60, autoBackoff: 360, airGrace: 60, airWindow: 7, airRetry: 60, maxGrabs: 5, seedRatioLimit: 1, seedTimeMinutes: 0, removeAfterImport: false });
  const form = useAppForm({ ...formBehaviour, validators: { onDynamic: schema }, defaultValues: defaults.current, onSubmit: async ({ value }) => {
    const values = schema.parse(value);
    // Only what was edited is written, so saving one panel does not create rows for every
    // default on the page.
    const baseline = defaults.current;
    const edited = (fields: Array<[keyof Values, string]>) => Object.fromEntries(fields.filter(([field]) => !Object.is(values[field], baseline[field])).map(([field, key]) => [key, values[field]]));
    await torrent.save(edited([["downloadDir", "torrent.download_dir"], ["sessionDir", "torrent.session_dir"], ["listenPort", "torrent.listen_port"], ["enableDht", "torrent.enable_dht"], ["maxConcurrent", "torrent.max_concurrent"], ["downloadLimit", "torrent.download_limit"], ["uploadLimit", "torrent.upload_limit"], ["seedRatioLimit", "torrent.seed_ratio_limit"], ["seedTimeMinutes", "torrent.seed_time_minutes"], ["removeAfterImport", "torrent.remove_after_import"]]));
    await auto.save(edited([["autoEnabled", "auto_download.enabled"], ["autoInterval", "auto_download.interval_minutes"], ["autoBackoff", "auto_download.search_backoff_minutes"], ["airGrace", "auto_download.air_grace_minutes"], ["airWindow", "auto_download.air_window_days"], ["airRetry", "auto_download.air_retry_minutes"], ["maxGrabs", "auto_download.max_concurrent_grabs"]]));
    defaults.current = values;
    toast.success("Settings saved");
  } });

  useEffect(() => {
    const t = torrent.values;
    const a = auto.values;
    // Initial category responses and live snapshots can arrive independently.
    // Update untouched fields/defaults without discarding an in-progress edit.
    const snapshot: Values = {
      downloadDir: t.get("torrent.download_dir") ?? "",
      sessionDir: t.get("torrent.session_dir") ?? "",
      listenPort: asNumber(t.get("torrent.listen_port"), 6881),
      enableDht: asBool(t.get("torrent.enable_dht"), true),
      maxConcurrent: asNumber(t.get("torrent.max_concurrent"), 5),
      downloadLimit: asNumber(t.get("torrent.download_limit"), 0),
      uploadLimit: asNumber(t.get("torrent.upload_limit"), 0),
      seedRatioLimit: asNumber(t.get("torrent.seed_ratio_limit"), 1),
      seedTimeMinutes: asNumber(t.get("torrent.seed_time_minutes"), 0),
      removeAfterImport: asBool(t.get("torrent.remove_after_import"), false),
      // Off until someone turns it on, as the server has it (download_monitor.rs).
      autoEnabled: asBool(a.get("auto_download.enabled"), false),
      autoInterval: asNumber(a.get("auto_download.interval_minutes"), 60),
      autoBackoff: asNumber(a.get("auto_download.search_backoff_minutes"), 360),
      airGrace: asNumber(a.get("auto_download.air_grace_minutes"), 60),
      airWindow: asNumber(a.get("auto_download.air_window_days"), 7),
      airRetry: asNumber(a.get("auto_download.air_retry_minutes"), 60),
      maxGrabs: asNumber(a.get("auto_download.max_concurrent_grabs"), 5),
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
    <div className="flex flex-col gap-6">
      <Form form={form} className="flex min-w-0 flex-col gap-6" onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))}>
        <Panel
          title="Automatic downloads"
          description="Librarian watches for what you want and sends the best match to the download client, on a schedule and from your RSS feeds."
          actions={<Button size="sm" variant="ghost" onPress={() => void runNow()} isPending={triggering}><IconPlay size="md" /> Run now</Button>}
        >
          <div className="flex flex-col gap-4">
            <form.AppField name="autoEnabled">{(field) => <field.SwitchRow label="Search and download automatically" description="Off by default. While off, nothing searches, polls a feed or downloads on its own; Search now and Run now still work." />}</form.AppField>
            <AutoDownloadStatus settings={auto.values} />
            <FieldGroup columns={2}>
              <form.AppField name="autoInterval">{(field) => <field.NumberField label="Check everything every (minutes)" min={5} max={1440} step={5} hint="A full sweep of every wanted title." />}</form.AppField>
              <form.AppField name="autoBackoff">{(field) => <field.NumberField label="Search a missing item again after (minutes)" min={5} max={10080} step={30} hint="Keeps one missing item from being searched on every sweep." />}</form.AppField>
              <form.AppField name="airGrace">{(field) => <field.NumberField label="Wait after an episode airs (minutes)" min={0} max={1440} step={15} hint="Releases take a while to appear. Nothing is searched for before this has passed." />}</form.AppField>
              <form.AppField name="airWindow">{(field) => <field.NumberField label="Watch new episodes for (days)" min={0} max={60} hint="How long a new episode is searched for on the quicker schedule. 0 skips it." />}</form.AppField>
              <form.AppField name="airRetry">{(field) => <field.NumberField label="Search a new episode again after (minutes)" min={5} max={1440} step={15} hint="Used until the days above run out; the setting before it applies after." />}</form.AppField>
              <form.AppField name="maxGrabs">{(field) => <field.NumberField label="Most automatic downloads at once" min={1} max={100} hint="Anything more waits for a free slot. Searching by hand is not limited." />}</form.AppField>
            </FieldGroup>
            <AutoDownloadGuide />
          </div>
        </Panel>
        <Panel title="Torrent client" description="The client reads these settings when it starts, so every change on this page, including seeding, takes effect after the server restarts.">
          <FieldGroup columns={2}>
            <form.AppField name="downloadDir">{(field) => <field.TextField label="Download folder" className="font-mono" required />}</form.AppField>
            <form.AppField name="sessionDir">{(field) => <field.TextField label="Session folder" className="font-mono" required hint="Resume data and DHT state." />}</form.AppField>
            <form.AppField name="listenPort">{(field) => <field.NumberField label="Listen port" min={1024} max={65535} />}</form.AppField>
            <form.AppField name="maxConcurrent">{(field) => <field.NumberField label="Concurrent downloads" min={1} max={50} />}</form.AppField>
            <form.AppField name="downloadLimit">{(field) => <field.NumberField label="Download limit (KB/s, 0 = unlimited)" min={0} />}</form.AppField>
            <form.AppField name="uploadLimit">{(field) => <field.NumberField label="Upload limit (KB/s, 0 = unlimited)" min={0} />}</form.AppField>
            <form.AppField name="enableDht">{(field) => <field.SwitchRow label="Enable DHT" description="Find peers without a tracker. Turn off for private-tracker-only setups." className="sm:col-span-2" />}</form.AppField>
          </FieldGroup>
        </Panel>
        <Panel title="Seeding" description="When a finished download stops sharing.">
          <FieldGroup columns={2}>
            <form.AppField name="seedRatioLimit">{(field) => <field.NumberField label="Seed until ratio" min={0} max={100} step={0.1} hint="0 = no ratio rule." />}</form.AppField>
            <form.AppField name="seedTimeMinutes">{(field) => <field.NumberField label="Seed for (minutes)" min={0} step={30} hint="0 = no time rule." />}</form.AppField>
            <form.AppField name="removeAfterImport">{(field) => <field.SwitchRow label="Remove the torrent and its files after import" description="Off keeps the files and only stops seeding." className="sm:col-span-2" />}</form.AppField>
          </FieldGroup>
        </Panel>
        <FormActions>
          <form.Subscribe selector={(state) => state.isSubmitting}>{(pending) => <form.CancelButton disabled={pending}>Discard</form.CancelButton>}</form.Subscribe>
          <form.SubmitButton>Save</form.SubmitButton>
        </FormActions>
      </Form>
      <BlockedReleases />
    </div>
  );
}

/** What each choice on a show, album, audiobook or movie does once automatic downloading is on. */
function AutoDownloadGuide() {
  const rows: Array<[string, string]> = [
    ["Shows, albums and audiobooks", `Set per title.  Off: ${MODE_HELP.NONE}  Wanted only: ${MODE_HELP.WANTED}  Everything missing: ${MODE_HELP.ALL}`],
    ["Movies", "A movie is downloaded when it is monitored and wanted, once its release date has passed."],
    ["New episodes", "Searched after they air plus the wait above, then on the quicker schedule for the days above, then on the ordinary one. Your RSS feeds are read as well and usually find them first."],
    ["Always", "Only the best release your quality profile accepts is taken; blocklisted releases are never taken again. Each search, grab and reason for skipping appears under Activity."],
  ];
  return (
    <dl className="flex flex-col gap-2 text-label-sm text-muted" aria-label="What automatic downloads do">
      {rows.map(([term, text]) => (
        <div key={term}>
          <dt className="font-medium text-foreground">{term}</dt>
          <dd>{text}</dd>
        </div>
      ))}
    </dl>
  );
}
