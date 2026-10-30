import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { IconPlayerPlay } from "@tabler/icons-react";
import { useEffect, useRef } from "react";
import { z } from "zod";

import { Button, FieldGroup, Panel } from "@/components/ui";
import { TriggerAutoDownloadDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

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
  retryAfter: requiredNumber(z.number().int().min(5).max(10080)),
  seedRatioLimit: requiredNumber(z.number().min(0).max(100)),
  seedTimeMinutes: requiredNumber(z.number().int().min(0).max(100000)),
  removeAfterImport: z.boolean(),
});
type Values = z.input<typeof schema>;

export function DownloadsSettings() {
  const torrent = useAppSettings("torrent");
  const auto = useAppSettings("auto_download");
  const [trigger, { loading: triggering }] = useMutation(TriggerAutoDownloadDocument);
  const defaults = useRef<Values>({ downloadDir: "", sessionDir: "", listenPort: 6881, enableDht: true, maxConcurrent: 5, downloadLimit: 0, uploadLimit: 0, autoEnabled: true, autoInterval: 60, retryAfter: 240, seedRatioLimit: 1, seedTimeMinutes: 0, removeAfterImport: false });
  const form = useAppForm({ ...formBehaviour, validators: { onDynamic: schema }, defaultValues: defaults.current, onSubmit: async ({ value }) => {
    const values = schema.parse(value);
    await torrent.save({ "torrent.download_dir": values.downloadDir, "torrent.session_dir": values.sessionDir, "torrent.listen_port": values.listenPort, "torrent.enable_dht": values.enableDht, "torrent.max_concurrent": values.maxConcurrent, "torrent.download_limit": values.downloadLimit, "torrent.upload_limit": values.uploadLimit, "torrent.seed_ratio_limit": values.seedRatioLimit, "torrent.seed_time_minutes": values.seedTimeMinutes, "torrent.remove_after_import": values.removeAfterImport });
    await auto.save({ "auto_download.enabled": values.autoEnabled, "auto_download.interval_minutes": values.autoInterval, "auto_download.retry_after_minutes": values.retryAfter });
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
      autoEnabled: asBool(a.get("auto_download.enabled"), true),
      autoInterval: asNumber(a.get("auto_download.interval_minutes"), 60),
      retryAfter: asNumber(a.get("auto_download.retry_after_minutes"), 240),
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
          description="Librarian searches your sources for anything wanted and sends the best match to the download client."
          actions={<Button size="sm" variant="ghost" onPress={() => void runNow()} isPending={triggering}><IconPlayerPlay size={16} /> Run now</Button>}
        >
          <div className="flex flex-col gap-4">
            <form.AppField name="autoEnabled">{(field) => <field.SwitchRow label="Search and download automatically" description="Turn this off to keep every grab manual." />}</form.AppField>
            <FieldGroup columns={2}>
              <form.AppField name="autoInterval">{(field) => <field.NumberField label="Check every (minutes)" min={5} max={1440} step={5} />}</form.AppField>
              <form.AppField name="retryAfter">{(field) => <field.NumberField label="Retry failed searches after (minutes)" min={5} max={10080} step={5} />}</form.AppField>
            </FieldGroup>
          </div>
        </Panel>
        <Panel title="Torrent client" description="Changes to folders and ports apply after the client restarts.">
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
