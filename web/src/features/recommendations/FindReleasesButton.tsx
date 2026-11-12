import { CountBadge } from "glass-ux/badge";
import { IconDownload } from "glass-ux/icons";

import { Button } from "@/components/ui";

/**
 * "Find releases", with a count of the releases Librarian recommends for this (a season, an album, a
 * movie...) on its corner. The count is in the button's name too, so a screen reader and the TV
 * remote's focus ring both say it. No badge when there is nothing to recommend.
 */
export function FindReleasesButton({ count, onPress, label = "Find releases", size = "md", variant = "secondary" }: {
  count: number;
  onPress: () => void;
  label?: string;
  size?: "sm" | "md" | "lg";
  variant?: "primary" | "secondary" | "ghost";
}) {
  const name = count > 0 ? `${label}, ${count} recommended` : label;
  return (
    <span className="relative inline-flex shrink-0">
      <Button size={size} variant={variant} onPress={onPress} aria-label={name} data-focusable>
        <IconDownload size="sm" /> {label}
      </Button>
      {count > 0 ? (
        <CountBadge value={count} tone="accent" overlay aria-hidden className="pointer-events-none absolute -end-1.5 -top-1.5" />
      ) : null}
    </span>
  );
}
