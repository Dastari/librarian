import { useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

import { useRefraction } from "@/lib/refraction";
import { cn } from "@/lib/utils";

/*
 * Liquid-glass form controls: a segmented control with a sliding refractive pill, a switch
 * with a glass thumb, and a slider. They are fully keyboard and remote operable and share the
 * recipes in styles/glass.css.
 */

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
  const ref = useRef<HTMLDivElement>(null);
  const [pill, setPill] = useState<{ left: number; width: number } | null>(null);
  const filter = useRefraction(ref, { depth: 5, strength: 30, blur: 4, brightness: 1.05 });

  useLayoutEffect(() => {
    const list = ref.current;
    if (!list) return;
    const measure = () => {
      const active = list.querySelector<HTMLElement>("[aria-checked='true']");
      setPill(active ? { left: active.offsetLeft, width: active.offsetWidth } : null);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(list);
    return () => observer.disconnect();
  }, [value, items]);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    const index = items.findIndex((item) => item.key === value);
    const next = items[(index + (event.key === "ArrowRight" ? 1 : -1) + items.length) % items.length];
    if (!next) return;
    event.preventDefault();
    onChange(next.key);
    ref.current?.querySelector<HTMLElement>(`[data-key='${next.key}']`)?.focus();
  };

  return (
    <div ref={ref} role="radiogroup" aria-label={ariaLabel} onKeyDown={onKeyDown} data-spatial-ignore className={cn("glass-track relative inline-flex shrink-0 self-start items-center rounded-pill p-1", className)} style={filter ? { backdropFilter: filter, WebkitBackdropFilter: filter } : undefined}>
      {pill ? <span aria-hidden className="glass-pill-active absolute top-1 bottom-1 rounded-pill transition-[left,width] duration-base ease-fluid" style={{ left: pill.left, width: pill.width }} /> : null}
      {items.map((item) => {
        const active = item.key === value;
        return (
          <button
            key={item.key}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={item.ariaLabel}
            data-key={item.key}
            data-focusable
            tabIndex={active ? 0 : -1}
            onClick={() => onChange(item.key)}
            className={cn(
              "nav-focus relative z-[1] inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-pill transition-colors duration-fast",
              size === "sm" ? "h-7 px-2.5 text-label" : "h-8 px-3.5 text-label",
              active ? "text-foreground" : "text-muted hover:text-foreground",
              item.icon && !item.label && "aspect-square px-0",
            )}
          >
            {item.icon}
            {item.label}
          </button>
        );
      })}
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
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      data-focusable
      onClick={() => onChange(!checked)}
      className={cn(
        "nav-focus glass-track relative h-7 w-12 shrink-0 rounded-pill transition-colors duration-base disabled:opacity-50",
        checked && "glass-brand",
        className,
      )}
    >
      <span className={cn("glass-thumb absolute top-0.5 left-0.5 size-6 rounded-full transition-transform duration-base ease-snap", checked && "translate-x-5")} />
    </button>
  );
}
