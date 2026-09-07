import { Button as AriaButton, type ButtonProps as AriaButtonProps } from "react-aria-components";
import { useRef, type ReactNode } from "react";

import { supportsRefraction, useRefraction } from "@/lib/refraction";
import { cn } from "@/lib/utils";

interface GlassButtonProps extends Omit<AriaButtonProps, "children" | "className"> {
  children?: ReactNode;
  className?: string;
  size?: "sm" | "md" | "lg" | "xl";
  /** Solid brand fill for the one primary action over artwork (Play). */
  emphasis?: "glass" | "brand";
  isIconOnly?: boolean;
  /** Use the refractive filter (a few large buttons over artwork), not for dense lists. */
  refract?: boolean;
}

const SIZE: Record<NonNullable<GlassButtonProps["size"]>, string> = {
  sm: "h-9 min-w-9 px-3 text-label gap-1.5 [&_svg]:size-4",
  md: "h-11 min-w-11 px-4 text-body-sm font-semibold gap-2 [&_svg]:size-5",
  lg: "h-13 min-w-13 px-6 text-body font-semibold gap-2.5 [&_svg]:size-6",
  xl: "h-16 min-w-16 px-8 text-body-lg font-semibold gap-3 [&_svg]:size-7",
};

/**
 * Pill button for use over artwork: hero banners, the player and poster overlays.
 * Uses the flat glass treatment because it is rendered many times per screen.
 */
export function GlassButton({ children, className, size = "md", emphasis = "glass", isIconOnly, refract = false, ...props }: GlassButtonProps) {
  const ref = useRef<HTMLButtonElement>(null);
  const filter = useRefraction(ref, { depth: 6, strength: 40, blur: 4 }, refract && emphasis === "glass");
  const refracting = refract && emphasis === "glass" && supportsRefraction;
  return (
    <AriaButton
      {...props}
      ref={ref}
      style={refracting ? { backdropFilter: filter, WebkitBackdropFilter: filter } : undefined}
      data-focusable
      className={cn(
        "nav-focus inline-flex shrink-0 items-center justify-center rounded-pill transition-[transform,background-color,box-shadow] duration-fast ease-fluid",
        "hover-capable:hover:-translate-y-px pressed:translate-y-0 pressed:scale-[0.98] disabled:pointer-events-none disabled:opacity-50",
        emphasis === "glass" ? "glass-control text-foreground hover-capable:hover:brightness-110" : "glass-control glass-brand hover-capable:hover:brightness-105",
        SIZE[size],
        isIconOnly && "px-0 aspect-square",
        className,
      )}
    >
      {children}
    </AriaButton>
  );
}
