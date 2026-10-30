import { useAppearance } from "glass-ux/appearance";
import { Segmented } from "glass-ux/segmented";
import { SettingRow } from "glass-ux/setting-row";
import { Slider } from "glass-ux/slider";
import { SwatchPicker } from "glass-ux/swatch-picker";

import { cn } from "@/lib/utils";

const palettes = [
  { value: "cinema", label: "Cinema", palette: "cinema" },
  { value: "midnight", label: "Midnight", palette: "midnight" },
  { value: "ember", label: "Ember", palette: "ember" },
  { value: "forest", label: "Forest", palette: "forest" },
];

/** Device-local appearance, backed by the library's persisted runtime. */
export function ThemePicker({ className }: { className?: string }) {
  const { appearance, update } = useAppearance();
  return (
    <div className={cn("flex flex-col gap-3", className)}>
      <SettingRow label="Palette" control={<SwatchPicker options={palettes} value={appearance.palette} onValueChange={(palette) => update({ palette })} collapse="none" />} />
      <SettingRow label="Color mode" control={<Segmented options={[{ value: "system", label: "System" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]} value={appearance.mode} onValueChange={(mode) => update({ mode })} />} />
      <SettingRow label="Glass" description="Choose the amount of transparency." control={<Segmented options={[{ value: "full", label: "Full" }, { value: "reduced", label: "Reduced" }, { value: "solid", label: "Solid" }]} value={appearance.glass} onValueChange={(glass) => update({ glass })} />} />
      <SettingRow label="Density" control={<Segmented options={[{ value: "comfortable", label: "Comfortable" }, { value: "compact", label: "Compact" }]} value={appearance.density} onValueChange={(density) => update({ density })} />} />
      <SettingRow label="Motion" control={<Segmented options={[{ value: "system", label: "System" }, { value: "full", label: "Full" }, { value: "reduced", label: "Reduced" }]} value={appearance.motion} onValueChange={(motion) => update({ motion })} />} />
      <SettingRow label="Corner radius" value={`${Math.round(appearance.radiusScale * 100)}%`} control={<Slider aria-label="Corner radius" min={0} max={1.5} step={0.1} value={appearance.radiusScale} onValueChange={(value) => update({ radiusScale: Number(value) })} format={(value) => `${Math.round(value * 100)}%`} className="w-40" />} />
      <SettingRow label="Border width" value={`${appearance.borderWidth}px`} control={<Slider aria-label="Border width" min={0.5} max={2} step={0.25} value={appearance.borderWidth} onValueChange={(value) => update({ borderWidth: Number(value) })} unit="px" className="w-40" />} />
      <SettingRow label="Interface size" value={`${Math.round(appearance.uiScale * 100)}%`} control={<Slider aria-label="Interface size" min={0.875} max={1.25} step={0.025} value={appearance.uiScale} onValueChange={(value) => update({ uiScale: Number(value) })} format={(value) => `${Math.round(value * 100)}%`} className="w-40" />} />
    </div>
  );
}
