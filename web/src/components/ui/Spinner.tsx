import { ProgressRing } from "glass-ux/progress";

/** Temporary numeric-size bridge to the library's compact indeterminate progress. */
export function Spinner({ size = 20, className }: { size?: number; className?: string }) {
  return <ProgressRing value={null} aria-label="Loading" size={size <= 20 ? "sm" : "md"} className={className} />;
}
