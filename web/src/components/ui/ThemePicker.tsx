import { IconCheck, IconSunMoon } from "@tabler/icons-react";

import { THEMES, useTheme, type ThemePreference } from "@/lib/theme";
import { cn } from "@/lib/utils";

/** Theme swatches: a mini preview of canvas, accent and ambient colour per theme. */
export function ThemePicker({ className }: { className?: string }) {
  const { preference, setPreference } = useTheme();
  const options: Array<{ id: ThemePreference; label: string; swatch?: [string, string, string] }> = [...THEMES, { id: "system", label: "Match system" }];
  return (
    <div role="radiogroup" aria-label="Theme" className={cn("grid grid-cols-2 gap-3 sm:grid-cols-3", className)}>
      {options.map((option) => {
        const active = preference === option.id;
        return (
          <button
            key={option.id}
            type="button"
            role="radio"
            aria-checked={active}
            data-focusable
            onClick={() => setPreference(option.id)}
            className={cn("nav-focus glass-control relative flex items-center gap-3 rounded-card p-3 text-left transition-colors", active && "glass-brand")}
          >
            {option.swatch ? (
              <span className="relative size-10 shrink-0 overflow-hidden rounded-lg border border-white/15" style={{ background: option.swatch[0] }}>
                <span className="absolute inset-x-0 bottom-0 h-1/2" style={{ background: `radial-gradient(80% 100% at 30% 100%, ${option.swatch[2]}, transparent 70%)` }} />
                <span className="absolute right-1.5 top-1.5 size-3 rounded-full" style={{ background: option.swatch[1] }} />
              </span>
            ) : (
              <span className="grid size-10 shrink-0 place-items-center rounded-lg bg-surface-secondary text-muted">
                <IconSunMoon size={20} />
              </span>
            )}
            <span className="min-w-0 flex-1 text-body-sm">{option.label}</span>
            {active ? <IconCheck size={16} className="shrink-0" /> : null}
          </button>
        );
      })}
    </div>
  );
}
