import { clsx, type ClassValue } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

/**
 * Glass UX and our typography define their own type scale (`text-label-sm`, `text-title`,
 * `text-caption`…). Unregistered, tailwind-merge reads them as text colours, so
 * `cn("text-label-sm text-muted")` would drop the size. `text-numeric` is a font feature,
 * not a size, so it must merge with neither.
 */
const TYPE_SCALE = [
  "body", "body-lg", "body-sm", "caption", "display", "display-lg", "display-md", "display-xl", "headline",
  "label", "label-sm", "mono", "overline", "subtitle", "title", "title-lg", "title-md", "title-sm",
];
const twMerge = extendTailwindMerge<"font-numeric" | "text-effect">({
  extend: {
    classGroups: {
      "font-size": [{ text: TYPE_SCALE }],
      "font-numeric": ["text-numeric"],
      "text-effect": ["text-accent-gradient", "text-shadow-hero"],
    },
  },
});

/** Preserve the former helper's numeric tokens, including nested class arrays. */
function normalizeClassValue(value: ClassValue): ClassValue {
  if (value === 0 || typeof value === "bigint") return String(value);
  return Array.isArray(value) ? value.map(normalizeClassValue) : value;
}

/** Combine conditional classes and resolve conflicting Tailwind utilities. */
export function cn(...inputs: ClassValue[]): string | undefined {
  return twMerge(clsx(inputs.map(normalizeClassValue))) || undefined;
}

/** Narrow a nullable to a value; useful in `.filter(isPresent)` chains. */
export function isPresent<T>(value: T | null | undefined): value is T {
  return value !== null && value !== undefined;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

/** Stable, dependency-free id for optimistic rows and DOM ids. */
export function uid(prefix = "id"): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

export function firstLetter(value: string | null | undefined): string {
  const char = (value ?? "").trim().charAt(0).toUpperCase();
  return /[A-Z]/.test(char) ? char : "#";
}
