import { Form, FormActions, FormStack, formBehaviour, numberRule, useAppForm } from "glass-ux/form";
import { SettingRow } from "glass-ux/setting-row";
import { Switch } from "glass-ux/switch";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { Panel, ThemePicker } from "@/components/ui";
import { useIsAdmin } from "@/lib/auth/useSession";
import { errorMessage } from "@/lib/graphql/errors";
import { useInputMode } from "@/lib/input-mode";

import { asList, asNumber, useAppSettings } from "./useAppSettings";

const schema = z.object({
  playbackSyncInterval: numberRule({ required: true, integer: true, min: 5, max: 300, requiredMessage: "Enter a progress interval", rangeMessage: "Enter a whole number from 5 to 300" }),
  preferredLanguage: z.string().trim().max(10),
  subtitleLanguages: z.string().trim().max(200),
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
    preferredLanguage: metadata.values.get("metadata.preferred_language") ?? "en",
    subtitleLanguages: asList(subtitles.values.get("subtitles.preferred_languages")).join(", "),
  }), [playback.values, metadata.values, subtitles.values]);
  const loading = playback.loading || metadata.loading || subtitles.loading;
  // TanStack reapplies the supplied defaults when an untouched form rerenders.
  // Keep those options aligned with every snapshot, save or discard we adopt.
  const [baseline, setBaseline] = useState<Values>(storedValues);

  const form = useAppForm({
    ...formBehaviour,
    defaultValues: baseline,
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      // TanStack validates but does not apply schema transforms to the submitted value.
      const saved = schema.parse(value);
      await playback.save({ playback_sync_interval: String(saved.playbackSyncInterval) });
      await metadata.save({ "metadata.preferred_language": saved.preferredLanguage });
      await subtitles.save({ "subtitles.preferred_languages": JSON.stringify(saved.subtitleLanguages.split(",").map((item) => item.trim()).filter(Boolean)) });
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
    <div className="flex flex-col gap-6">
      <Panel title="Appearance" description="Applies to this device.">
        <ThemePicker />
        <SettingRow
          label="TV mode"
          description={`Larger targets and type for remote controls. Currently ${mode === "tv" ? "on" : "off"}${forced ? " (forced)" : " (detected)"}.`}
          control={<Switch checked={forced === "tv"} onCheckedChange={(checked) => setForced(checked ? "tv" : null)} />}
          className="mt-3"
        />
      </Panel>

      {isAdmin ? (
        <Form form={form} onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))} className="flex min-w-0 flex-col gap-6">
          <form.Subscribe selector={(state) => state.isSubmitting}>
            {(submitting) => <>
              <Panel title="Playback">
                <FormStack>
                  <form.AppField name="playbackSyncInterval">
                    {(field) => <field.NumberField label="Save progress every (seconds)" min={5} max={300} step={5} required disabled={loading || submitting} />}
                  </form.AppField>
                </FormStack>
              </Panel>
              <Panel title="Language">
                <FormStack>
                  <form.AppField name="preferredLanguage">
                    {(field) => <field.TextField label="Metadata language" hint="ISO code, e.g. en or de." disabled={loading || submitting} />}
                  </form.AppField>
                  <form.AppField name="subtitleLanguages">
                    {(field) => <field.TextField label="Preferred subtitle languages" hint="Comma separated, in order of preference." disabled={loading || submitting} />}
                  </form.AppField>
                </FormStack>
              </Panel>
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
    </div>
  );
}
