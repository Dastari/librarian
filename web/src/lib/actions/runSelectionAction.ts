import { errorMessage } from "@/lib/graphql/errors";

/** Run every selected target, refresh once, and let the shared runner report one outcome. */
export async function runSelectionAction<T>(rows: readonly T[], label: string, name: (row: T) => string, run: (row: T) => Promise<unknown>, onChanged?: () => void) {
  const failures: string[] = [];
  try {
    for (const row of rows) {
      try { await run(row); }
      catch (error) { failures.push(`${name(row)}: ${errorMessage(error)}`); }
    }
  } finally { onChanged?.(); }
  if (failures.length) throw new Error(`${label} succeeded for ${rows.length - failures.length} of ${rows.length} selected items. ${failures.length} failed. ${failures.join("; ")}`);
}
