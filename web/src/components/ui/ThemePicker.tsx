import { Segmented } from "glass-ux/segmented";
import { SettingRow } from "glass-ux/setting-row";
import { Switch } from "glass-ux/switch";
import { useSyncExternalStore } from "react";

import { animatedBackgroundsStore, animatedBackgroundsUnavailable, setAnimatedBackgrounds } from "@/lib/glass/appearance";
import { DISPLAY_SIZES, displaySizeStore, isTvApp, setDisplaySize } from "@/lib/tv-app";

import { THEMES, useTheme, type ThemePreference } from "@/lib/theme";
import { cn } from "@/lib/utils";

/**
 * The appearance choices left to make: Dark or Light, whether glass blurs and whether backgrounds
 * move. Librarian is otherwise always Cinema with full glass at the library's default geometry.
 */
export function ThemePicker({ className }: { className?: string }) {
  const { preference, setPreference, blur, setBlur } = useTheme();
  const animated = useSyncExternalStore(animatedBackgroundsStore.subscribe, animatedBackgroundsStore.getSnapshot, animatedBackgroundsStore.getSnapshot);
  const unavailable = animatedBackgroundsUnavailable();
  const displaySize = useSyncExternalStore(displaySizeStore.subscribe, displaySizeStore.getSnapshot, displaySizeStore.getSnapshot);
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <SettingRow
        label="Color mode"
        control={<Segmented options={THEMES.map((theme) => ({ value: theme.id, label: theme.label }))} value={preference} onValueChange={(mode) => setPreference(mode as ThemePreference)} />}
      />
      <SettingRow
        label="Blur"
        description="Blur what shows through glass. Turn it off for smoother scrolling on slower devices. Applies to everyone using this device."
        control={<Switch aria-label="Blur" checked={blur} onCheckedChange={setBlur} />}
      />
      <SettingRow
        label="Animated backgrounds"
        description={unavailable ?? "The moving scene behind Home and sign-in. Turn it off to save memory and battery. Applies to everyone using this device."}
        control={<Switch aria-label="Animated backgrounds" checked={animated} disabled={unavailable !== null} onCheckedChange={setAnimatedBackgrounds} />}
      />
      {isTvApp() ? (
        // TVs differ in size, distance and the density they report, so the TV app alone may scale
        // its 10-foot canvas (lib/tv-app.ts). Kept on this TV, not in the profile.
        <SettingRow
          label="Display size"
          description="Make everything larger or smaller on this TV."
          control={<Segmented aria-label="Display size" options={DISPLAY_SIZES.map((size) => ({ value: String(size), label: `${size}%` }))} value={String(displaySize)} onValueChange={(size) => setDisplaySize(Number(size))} />}
        />
      ) : null}
    </div>
  );
}
