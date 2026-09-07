import { useMutation } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { IconPlayerPlay } from "@tabler/icons-react";
import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, FieldGroup, FormNumberField, FormSwitchField, FormTextField, Panel } from "@/components/ui";
import { TriggerAutoDownloadDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

import { BlockedReleases } from "./BlockedReleases";
import { SettingsForm } from "./SettingsForm";
import { asBool, asNumber, useAppSettings } from "./useAppSettings";

const schema = z.object({
  downloadDir: z.string().trim().min(1, "Choose a download folder"),
  sessionDir: z.string().trim().min(1, "Choose a session folder"),
  listenPort: z.number().int().min(1024).max(65535),
  enableDht: z.boolean(),
  maxConcurrent: z.number().int().min(1).max(50),
  downloadLimit: z.number().int().min(0),
  uploadLimit: z.number().int().min(0),
  autoEnabled: z.boolean(),
  autoInterval: z.number().int().min(5).max(1440),
  retryAfter: z.number().int().min(5).max(10080),
  seedRatioLimit: z.number().min(0).max(100),
  seedTimeMinutes: z.number().int().min(0).max(100000),
  removeAfterImport: z.boolean(),
});
type Values = z.infer<typeof schema>;

export function DownloadsSettings() {
  const torrent = useAppSettings("torrent");
  const auto = useAppSettings("auto_download");
  const [trigger, { loading: triggering }] = useMutation(TriggerAutoDownloadDocument);
  const form = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { downloadDir: "", sessionDir: "", listenPort: 6881, enableDht: true, maxConcurrent: 5, downloadLimit: 0, uploadLimit: 0, autoEnabled: true, autoInterval: 60, retryAfter: 240, seedRatioLimit: 1, seedTimeMinutes: 0, removeAfterImport: false } });

  useEffect(() => {
    const t = torrent.values;
    const a = auto.values;
    form.reset({
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
    });
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
      <SettingsForm
        form={form}
        onSave={async (values) => {
          await torrent.save({ "torrent.download_dir": values.downloadDir, "torrent.session_dir": values.sessionDir, "torrent.listen_port": values.listenPort, "torrent.enable_dht": values.enableDht, "torrent.max_concurrent": values.maxConcurrent, "torrent.download_limit": values.downloadLimit, "torrent.upload_limit": values.uploadLimit, "torrent.seed_ratio_limit": values.seedRatioLimit, "torrent.seed_time_minutes": values.seedTimeMinutes, "torrent.remove_after_import": values.removeAfterImport });
          await auto.save({ "auto_download.enabled": values.autoEnabled, "auto_download.interval_minutes": values.autoInterval, "auto_download.retry_after_minutes": values.retryAfter });
        }}
      >
        <Panel
          title="Automatic downloads"
          description="Librarian searches your sources for anything wanted and sends the best match to the download client."
          actions={<Button size="sm" variant="ghost" onPress={() => void runNow()} isPending={triggering}><IconPlayerPlay size={16} /> Run now</Button>}
        >
          <div className="flex flex-col gap-4">
            <FormSwitchField control={form.control} name="autoEnabled" label="Search and download automatically" description="Turn this off to keep every grab manual." />
            <FieldGroup columns={2}>
              <FormNumberField control={form.control} name="autoInterval" label="Check every (minutes)" min={5} max={1440} step={5} />
              <FormNumberField control={form.control} name="retryAfter" label="Retry failed searches after (minutes)" min={5} max={10080} step={5} />
            </FieldGroup>
          </div>
        </Panel>
        <Panel title="Torrent client" description="Changes to folders and ports apply after the client restarts.">
          <FieldGroup columns={2}>
            <FormTextField control={form.control} name="downloadDir" label="Download folder" mono isRequired />
            <FormTextField control={form.control} name="sessionDir" label="Session folder" mono isRequired description="Resume data and DHT state." />
            <FormNumberField control={form.control} name="listenPort" label="Listen port" min={1024} max={65535} />
            <FormNumberField control={form.control} name="maxConcurrent" label="Concurrent downloads" min={1} max={50} />
            <FormNumberField control={form.control} name="downloadLimit" label="Download limit (KB/s, 0 = unlimited)" min={0} />
            <FormNumberField control={form.control} name="uploadLimit" label="Upload limit (KB/s, 0 = unlimited)" min={0} />
            <FormSwitchField control={form.control} name="enableDht" label="Enable DHT" description="Find peers without a tracker. Turn off for private-tracker-only setups." className="sm:col-span-2" />
          </FieldGroup>
        </Panel>
        <Panel title="Seeding" description="When a finished download stops sharing.">
          <FieldGroup columns={2}>
            <FormNumberField control={form.control} name="seedRatioLimit" label="Seed until ratio" min={0} max={100} step={0.1} description="0 = no ratio rule." />
            <FormNumberField control={form.control} name="seedTimeMinutes" label="Seed for (minutes)" min={0} step={30} description="0 = no time rule." />
            <FormSwitchField control={form.control} name="removeAfterImport" label="Remove the torrent and its files after import" description="Off keeps the files and only stops seeding." className="sm:col-span-2" />
          </FieldGroup>
        </Panel>
      </SettingsForm>
      <BlockedReleases />
    </div>
  );
}
