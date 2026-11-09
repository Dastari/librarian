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
import { LANGUAGES } from "@/lib/languages";

import { DurationRow, NumberRow, TagsRow, TextRow, settingsFormBehaviour } from "./SettingFields";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { asList, asNumber, useAppSettings } from "./useAppSettings";

const schema = z.object({
  playbackSyncInterval: numberRule({ required: true, integer: true, min: 5, max: 300, requiredMessage: "Enter a progress interval", rangeMessage: "Enter a whole number from 5 to 300" }),
  historyRetentionDays: numberRule({ required: true, integer: true, min: 0, max: 36500, requiredMessage: "Enter a number of days, or 0 to keep everything", rangeMessage: "Enter a whole number of days from 0 to 36500" }),
  preferredLanguage: z.string().trim().max(10),
  subtitleLanguages: z.array(z.string().trim().min(1)).refine((languages) => languages.join(",").length <= 200, "Use at most 200 characters"),
});
type Values = z.infer<typeof schema>;

/** Per-device appearance plus a few server-wide playback defaults (admins). */
export function GeneralSettings() {
  const isAdmin = useIsAdmin();
  const { forced, setForced, mode } = useInputMode();
  const playback = useAppSettings("playback");
  const metadata = useAppSettings("metadata");
  const subtitles = useAppSettings("subtitles");
  const storedValues = useMemo<Values>(() => ({
    playbackSyncInterval: asNumber(playback.values.get("playback_sync_interval"), 15),
    historyRetentionDays: asNumber(playback.values.get("play_history.retention_days"), 0),
    preferredLanguage: metadata.values.get("metadata.preferred_language") ?? "en",
    subtitleLanguages: asList(subtitles.values.get("subtitles.preferred_languages")),
  }), [playback.values, metadata.values, subtitles.values]);
  const loading = playback.loading || metadata.loading || subtitles.loading;
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
      await playback.save({ playback_sync_interval: String(saved.playbackSyncInterval), "play_history.retention_days": String(saved.historyRetentionDays) });
      await metadata.save({ "metadata.preferred_language": saved.preferredLanguage });
      await subtitles.save({ "subtitles.preferred_languages": JSON.stringify(saved.subtitleLanguages) });
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
          description={`Larger targets and type for remote controls. Currently ${mode === "tv" ? "on" : "off"}${forced ? " (forced)" : " (detected)"}.`}
          control={<Switch checked={forced === "tv"} onCheckedChange={(checked) => setForced(checked ? "tv" : null)} />}
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
                <form.AppField name="historyRetentionDays">
                  {() => <NumberRow label="Keep play history for" description="Everyone's plays older than this are deleted, a few times a day. 0 keeps them all." unit="days" min={0} max={36500} disabled={loading || submitting} />}
                </form.AppField>
              </SettingsSection>
              <SettingsSection title="Language" width="readable">
                <form.AppField name="preferredLanguage">
                  {() => <TextRow label="Metadata language" description="ISO code, e.g. en or de." disabled={loading || submitting} />}
                </form.AppField>
                <form.AppField name="subtitleLanguages">
                  {() => <TagsRow label="Preferred subtitle languages" description="In order of preference." suggestions={LANGUAGES.map((language) => ({ value: language.code, label: language.name }))} disabled={loading || submitting} />}
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
