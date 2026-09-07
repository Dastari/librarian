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
      className={cn(
        "scrollbar-none flex text-label-sm",
        orientation === "vertical" ? "flex-col items-center gap-px" : "flex-row gap-0.5 overflow-x-auto",
        className,
      )}
    >
      {LETTERS.map((letter) => {
        const active = value === letter;
        const has = !available || available.has(letter);
        return (
          <button
            key={letter}
            type="button"
            data-focusable
            aria-pressed={active}
            onClick={() => onChange(letter)}
            className={cn(
              "nav-focus grid size-6 place-items-center rounded-md transition-colors duration-fast",
              active ? "bg-brand text-brand-foreground" : has ? "text-muted hover:bg-surface-hover hover:text-foreground" : "text-muted/40",
            )}
          >
            {letter}
          </button>
        );
      })}
    </div>
  );
}
