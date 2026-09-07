import { zodResolver } from "@hookform/resolvers/zod";
import { IconDeviceTv } from "@tabler/icons-react";
import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { FieldGroup, FormNumberField, FormTextField, Panel, ThemePicker } from "@/components/ui";
import { useIsAdmin } from "@/lib/auth/useSession";
import { useInputMode } from "@/lib/input-mode";
import { cn } from "@/lib/utils";

import { SettingsForm } from "./SettingsForm";
import { asList, asNumber, useAppSettings } from "./useAppSettings";

const schema = z.object({
  playbackSyncInterval: z.number().int().min(5).max(300),
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

  const form = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { playbackSyncInterval: 15, preferredLanguage: "en", subtitleLanguages: "" } });
  useEffect(() => {
    form.reset({
      playbackSyncInterval: asNumber(playback.values.get("playback_sync_interval"), 15),
      preferredLanguage: metadata.values.get("metadata.preferred_language") ?? "en",
      subtitleLanguages: asList(subtitles.values.get("subtitles.preferred_languages")).join(", "),
    });
  }, [playback.values, metadata.values, subtitles.values, form]);

  return (
    <div className="flex flex-col gap-6">
      <Panel title="Appearance" description="Applies to this device.">
        <ThemePicker />
        <button
          type="button"
          data-focusable
          onClick={() => setForced(forced === "tv" ? null : "tv")}
          aria-pressed={forced === "tv"}
          className={cn("nav-focus glass-control mt-3 flex w-full items-center gap-3 rounded-card p-4 text-left transition-colors", forced === "tv" && "glass-brand")}
        >
          <IconDeviceTv size={20} className="shrink-0" />
          <span className="min-w-0">
            <span className="block text-body-sm">TV mode</span>
            <span className="block text-label-sm opacity-75">Larger targets and type for remote controls. Currently {mode === "tv" ? "on" : "off"}{forced ? " (forced)" : " (detected)"}.</span>
          </span>
        </button>
      </Panel>

      {isAdmin ? (
        <SettingsForm
          form={form}
          onSave={async (values) => {
            await playback.save({ playback_sync_interval: String(values.playbackSyncInterval) });
            await metadata.save({ "metadata.preferred_language": values.preferredLanguage });
            await subtitles.save({ "subtitles.preferred_languages": JSON.stringify(values.subtitleLanguages.split(",").map((item) => item.trim()).filter(Boolean)) });
          }}
        >
          <Panel title="Playback">
            <FieldGroup columns={2}>
              <FormNumberField control={form.control} name="playbackSyncInterval" label="Save progress every (seconds)" min={5} max={300} step={5} />
            </FieldGroup>
          </Panel>
          <Panel title="Language">
            <FieldGroup columns={2}>
              <FormTextField control={form.control} name="preferredLanguage" label="Metadata language" description="ISO code, e.g. en or de." />
              <FormTextField control={form.control} name="subtitleLanguages" label="Preferred subtitle languages" description="Comma separated, in order of preference." />
            </FieldGroup>
          </Panel>
        </SettingsForm>
      ) : null}
    </div>
  );
}
