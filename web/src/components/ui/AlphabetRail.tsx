import { Button } from "glass-ux/button";

import { cn } from "@/lib/utils";

const LETTERS = ["#", ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ"];

interface AlphabetRailProps {
  /** Letter to highlight (the last jump target). */
  value: string | null;
  onChange: (letter: string | null) => void;
  /** Letters that have content; others are dimmed but still selectable. */
  available?: Set<string>;
  className?: string;
  orientation?: "vertical" | "horizontal";
}

/** Jump-to-letter control for large libraries (Plex style). Selecting a letter scrolls, it does not filter. */
export function AlphabetRail({ value, onChange, available, className, orientation = "vertical" }: AlphabetRailProps) {
  return (
    <div
      role="group"
      aria-label="Jump to letter"
      data-nav-zone="letters"
      data-nav-side={orientation === "horizontal" ? undefined : ""}
      className={cn(
        "scrollbar-none flex text-label-sm",
        orientation === "vertical" ? "flex-col items-center gap-px overflow-y-auto" : "flex-row gap-0.5 overflow-x-auto",
        className,
      )}
    >
      {LETTERS.map((letter) => {
        const active = value === letter;
        const has = !available || available.has(letter);
        return (
          <Button
            key={letter}
            type="button"
            size="xs"
            variant={active ? "primary" : "ghost"}
            aria-pressed={active}
            onClick={() => onChange(letter)}
            className={cn("shrink-0", !has && !active && "text-muted/40")}
            data-focusable
          >
            {letter}
          </Button>
        );
      })}
    </div>
  );
}
