import { byteSizeSchema } from "glass-ux/byte-size-field";
import { Form, FormActions, numberRule, useAppForm } from "glass-ux/form";
import { SettingRow } from "glass-ux/setting-row";
import { Switch } from "glass-ux/switch";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { ThemePicker } from "@/components/ui";
import { useIsAdmin } from "@/lib/auth/useSession";
import { errorMessage } from "@/lib/graphql/errors";
import { useInputMode } from "@/lib/input-mode";
import { isTvApp } from "@/lib/tv-app";

import { DEFAULT_SPACE_THRESHOLDS, SPACE_SETTING_KEYS, spaceThresholds } from "@/features/libraries/volumeSpace";

import { ByteSizeRow, DurationRow, NumberRow, PercentRow, TextRow, settingsFormBehaviour } from "./SettingFields";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { asBool, asNumber, useAppSettings } from "./useAppSettings";

const schema = z.object({
  playbackSyncInterval: numberRule({ required: true, integer: true, min: 5, max: 300, requiredMessage: "Enter a progress interval", rangeMessage: "Enter a whole number from 5 to 300" }),
  previewBackfill: z.boolean(),
  historyRetentionDays: numberRule({ required: true, integer: true, min: 0, max: 36500, requiredMessage: "Enter a number of days, or 0 to keep everything", rangeMessage: "Enter a whole number of days from 0 to 36500" }),
  preferredLanguage: z.string().trim().max(10),
  // The LAN announcement for TV apps (docs/design.md Q138); the server checks the address too.
  advertise: z.boolean(),
  // Low disk space alerts (docs/design.md Q146): the larger of the share and the size applies.
  warningFraction: numberRule({ required: true, min: 0, max: 1, requiredMessage: "Enter a share of the disk", rangeMessage: "Enter a share from 0 to 100 %" }),
  warningBytes: byteSizeSchema({ required: true, min: 0 }),
  criticalFraction: numberRule({ required: true, min: 0, max: 1, requiredMessage: "Enter a share of the disk", rangeMessage: "Enter a share from 0 to 100 %" }),
  criticalBytes: byteSizeSchema({ required: true, min: 0 }),
  webUrl: z.string().trim().max(240).refine((url) => url === "" || /^https?:\/\/[^\s/?#@]+(\/[^\s?#]*)?$/i.test(url), "Enter a full address such as http://192.168.1.20:3001, or leave it empty"),
});
type Values = z.infer<typeof schema>;
/** Seek-bar previews and black-bar detection for files that predate them (design.md Q124). */
const PREVIEW_BACKFILL_KEY = "playback.video_preview_backfill";

/** Per-device appearance plus a few server-wide playback defaults (admins). */
export function GeneralSettings() {
  const isAdmin = useIsAdmin();
  const { forced, setForced, mode } = useInputMode();
  const playback = useAppSettings("playback");
  const metadata = useAppSettings("metadata");
  const discovery = useAppSettings("discovery");
  const storage = useAppSettings("storage");
  const storedValues = useMemo<Values>(() => ({
    playbackSyncInterval: asNumber(playback.values.get("playback_sync_interval"), 15),
    previewBackfill: asBool(playback.values.get(PREVIEW_BACKFILL_KEY), true),
    historyRetentionDays: asNumber(playback.values.get("play_history.retention_days"), 0),
    preferredLanguage: metadata.values.get("metadata.preferred_language") ?? "en",
    advertise: asBool(discovery.values.get("discovery.advertise"), true),
    webUrl: discovery.values.get("discovery.web_url") ?? "",
    ...spaceThresholds(storage.values),
  }), [playback.values, metadata.values, discovery.values, storage.values]);
  const loading = playback.loading || metadata.loading || discovery.loading || storage.loading;
  // TanStack reapplies the supplied defaults when an untouched form rerenders.
  // Keep those options aligned with every snapshot, save or discard we adopt.
  const [baseline, setBaseline] = useState<Values>(storedValues);

  const form = useAppForm({
    ...settingsFormBehaviour,
    defaultValues: baseline,
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      // TanStack validates but does not apply schema transforms to the submitted value.
      const saved = schema.parse(value);
      await playback.save({ playback_sync_interval: String(saved.playbackSyncInterval), [PREVIEW_BACKFILL_KEY]: saved.previewBackfill, "play_history.retention_days": String(saved.historyRetentionDays) });
      await metadata.save({ "metadata.preferred_language": saved.preferredLanguage });
      await discovery.save({ "discovery.advertise": saved.advertise, "discovery.web_url": JSON.stringify(saved.webUrl) });
      // Required fields, so never empty here; critical is never looser than warning.
      const warningFraction = saved.warningFraction ?? DEFAULT_SPACE_THRESHOLDS.warningFraction;
      const warningBytes = saved.warningBytes ?? DEFAULT_SPACE_THRESHOLDS.warningBytes;
      await storage.save({
        [SPACE_SETTING_KEYS.warningFraction]: warningFraction,
        [SPACE_SETTING_KEYS.warningBytes]: warningBytes,
        [SPACE_SETTING_KEYS.criticalFraction]: Math.min(saved.criticalFraction ?? DEFAULT_SPACE_THRESHOLDS.criticalFraction, warningFraction),
        [SPACE_SETTING_KEYS.criticalBytes]: Math.min(saved.criticalBytes ?? DEFAULT_SPACE_THRESHOLDS.criticalBytes, warningBytes),
      });
      setBaseline(saved);
      formApi.reset(saved);
      toast.success("Settings saved");
    },
  });
  useEffect(() => {
    // A live snapshot may arrive between category saves. Keep the complete draft
    // until save finishes or the user discards it; discard uses the latest snapshot.
    if (!loading && form.state.isDefaultValue && !form.state.isSubmitting) {
      setBaseline(storedValues);
      form.reset(storedValues);
    }
  }, [storedValues, loading, form]);

  return (
    <SettingsPage>
      <SettingsSection title="This device" width="readable">
        <ThemePicker />
        <SettingRow
          label="TV mode"
          description={isTvApp() ? "Always on in the TV app." : `Larger targets and type for remote controls. Currently ${mode === "tv" ? "on" : "off"}${forced ? " (forced)" : " (detected)"}.`}
          control={<Switch checked={isTvApp() || forced === "tv"} disabled={isTvApp()} onCheckedChange={(checked) => setForced(checked ? "tv" : null)} />}
        />
      </SettingsSection>

      {isAdmin ? (
        <Form form={form} onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))} className="settings-measure flex min-w-0 flex-col gap-8">
          <form.Subscribe selector={(state) => state.isSubmitting}>
            {(submitting) => <>
              <SettingsSection title="Playback" width="readable">
                <form.AppField name="playbackSyncInterval">
                  {() => <DurationRow label="Save progress every" description="How often the position in what you are watching is saved." units={["m", "s"]} defaultUnit="s" disabled={loading || submitting} />}
                </form.AppField>
                <form.AppField name="previewBackfill">
                  {(field) => <field.SwitchRow label="Make previews for existing videos" description="Seek-bar previews and black-bar detection for videos added before them, a few at a time in the background. New and played videos always get them." disabled={loading || submitting} />}
                </form.AppField>
                <form.AppField name="historyRetentionDays">
                  {() => <NumberRow label="Keep play history for" description="Everyone's plays older than this are deleted, a few times a day. 0 keeps them all." unit="days" min={0} max={36500} disabled={loading || submitting} />}
                </form.AppField>
              </SettingsSection>
              <SettingsSection title="Storage" width="readable">
                <form.AppField name="warningFraction">
                  {() => <PercentRow label="Warning share" description="Warn when a disk or share has less free than this share of it, or than the warning size, whichever is larger. Admins and library owners get one alert per disk until it has room again." min={0} max={1} step={0.01} disabled={loading || submitting} />}
                </form.AppField>
                <form.AppField name="warningBytes">
                  {() => <ByteSizeRow label="Warning size" description="Free space in bytes, as disks are sold (50 GB)." defaultUnit="GB" disabled={loading || submitting} />}
                </form.AppField>
                <form.AppField name="criticalFraction">
                  {() => <PercentRow label="Critical share" description="Critically low below this share or the critical size, whichever is larger. Downloads that would not fit on a critical disk are refused." min={0} max={1} step={0.01} disabled={loading || submitting} />}
                </form.AppField>
                <form.AppField name="criticalBytes">
                  {() => <ByteSizeRow label="Critical size" description="Never more than the warning size." defaultUnit="GB" disabled={loading || submitting} />}
                </form.AppField>
              </SettingsSection>
              <SettingsSection title="Language" width="readable">
                <form.AppField name="preferredLanguage">
                  {() => <TextRow label="Metadata language" description="ISO code, e.g. en or de." disabled={loading || submitting} />}
                </form.AppField>
              </SettingsSection>
              <SettingsSection title="Network" width="readable">
                <form.AppField name="advertise">
                  {(field) => <field.SwitchRow label="Announce on the local network" description="Lets the Librarian TV app find this server by itself (mDNS, _librarian._tcp)." disabled={loading || submitting} />}
                </form.AppField>
                <form.AppField name="webUrl">
                  {() => <TextRow label="Address for TV apps" description="What a TV should open, such as http://192.168.1.20:3001. Leave empty to use this machine's network address." disabled={loading || submitting} mono />}
                </form.AppField>
              </SettingsSection>
              <FormActions>
                <form.Subscribe selector={(state) => state.isDefaultValue}>
                  {(unchanged) => <form.CancelButton disabled={loading || submitting || unchanged} onCancel={() => {
                    setBaseline(storedValues);
                    form.reset(storedValues);
                  }}>Discard</form.CancelButton>}
                </form.Subscribe>
                <form.SubmitButton disabled={loading || submitting}>Save</form.SubmitButton>
              </FormActions>
            </>}
          </form.Subscribe>
        </Form>
      ) : null}
    </SettingsPage>
  );
}
