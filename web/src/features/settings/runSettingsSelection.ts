import { errorMessage } from "@/lib/graphql/errors";

/** Keep processing independent targets and reconcile even when only some changes succeed. */
export async function runSettingsSelection<T>(
  rows: readonly T[],
  operation: string,
  label: (row: T) => string,
  run: (row: T) => Promise<unknown>,
  refresh?: () => unknown,
) {
  const failures: string[] = [];
  let refreshFailure: string | undefined;
  try {
    for (const row of rows) {
      try { await run(row); }
      catch (error) { failures.push(`${label(row)}: ${errorMessage(error)}`); }
    }
  } finally {
    try { await refresh?.(); }
    catch (error) { refreshFailure = errorMessage(error); }
  }
  if (failures.length) throw new Error(`${operation} succeeded for ${rows.length - failures.length} of ${rows.length}. ${failures.length} failed. ${failures.join("; ")}`);
  if (refreshFailure) throw new Error(`${operation} completed, but refreshing the results failed: ${refreshFailure}`);
}
