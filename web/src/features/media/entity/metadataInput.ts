/**
 * What changed in a metadata form, ready for an update mutation. Only changes are sent: a sort
 * title that wasn't touched has to keep following a new title. Blank text is sent as null, which
 * also hands a sort title back to the title.
 */
export function changedFields<T extends Record<string, unknown>>(saved: T, values: T): { [K in keyof T]?: T[K] | null } {
  const input: { [K in keyof T]?: T[K] | null } = {};
  for (const key of Object.keys(values) as Array<keyof T>) {
    if (JSON.stringify(saved[key]) === JSON.stringify(values[key])) continue;
    const value = values[key];
    input[key] = typeof value === "string" ? ((value.trim() || null) as T[typeof key] | null) : value;
  }
  return input;
}

/** A year typed into a form as the number the schema holds. */
export const yearValue = (year: string): number | null => (year.trim() ? Number(year) : null);
