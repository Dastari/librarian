import { cn } from "@/lib/utils";

/** Ring spinner drawn in the current text colour. */
export function Spinner({ size = 20, className }: { size?: number; className?: string }) {
  return (
    <svg role="status" aria-label="Loading" width={size} height={size} viewBox="0 0 24 24" fill="none" className={cn("animate-spin text-current", className)}>
      <circle cx="12" cy="12" r="9" stroke="currentColor" strokeOpacity="0.25" strokeWidth="2.5" />
      <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" />
    </svg>
  );
}
