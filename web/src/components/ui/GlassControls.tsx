import { Segmented } from "glass-ux/segmented";
import { Switch } from "glass-ux/switch";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export interface GlassSegment<TKey extends string> {
  key: TKey;
  label: ReactNode;
  icon?: ReactNode;
  ariaLabel?: string;
}

interface GlassSegmentedProps<TKey extends string> {
  items: Array<GlassSegment<TKey>>;
  value: TKey;
  onChange: (value: TKey) => void;
  ariaLabel: string;
  size?: "sm" | "md";
  className?: string;
}

export function GlassSegmented<TKey extends string>({ items, value, onChange, ariaLabel, size = "md", className }: GlassSegmentedProps<TKey>) {
  return (
    <div className={cn("min-w-0 max-w-full", className)}>
      <Segmented
        options={items.map((item) => ({ value: item.key, label: item.ariaLabel ?? (typeof item.label === "string" || typeof item.label === "number" ? String(item.label) : item.key), icon: item.icon }))}
        value={value}
        onValueChange={onChange}
        aria-label={ariaLabel}
        size={size}
        iconOnly={items.length > 0 && items.every((item) => !item.label && item.icon)}
      />
    </div>
  );
}

interface GlassSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  ariaLabel: string;
  disabled?: boolean;
  className?: string;
}

export function GlassSwitch({ checked, onChange, ariaLabel, disabled, className }: GlassSwitchProps) {
  return <Switch checked={checked} onCheckedChange={(checked) => onChange(checked)} aria-label={ariaLabel} disabled={disabled} className={className} data-focusable />;
}
