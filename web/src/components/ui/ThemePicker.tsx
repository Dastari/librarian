import { Segmented } from "glass-ux/segmented";
import { SettingRow } from "glass-ux/setting-row";
import { Switch } from "glass-ux/switch";

import { THEMES, useTheme, type ThemePreference } from "@/lib/theme";
import { cn } from "@/lib/utils";

/**
 * The appearance choices left to make: Dark or Light, and whether glass blurs. Librarian is
 * otherwise always Cinema with full glass at the library's default geometry.
 */
export function ThemePicker({ className }: { className?: string }) {
  const { preference, setPreference, blur, setBlur } = useTheme();
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
    </div>
  );
}
