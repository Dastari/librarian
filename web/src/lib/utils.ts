import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

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
