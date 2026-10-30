import { Button } from "@/components/ui";

/** Keep the previous complete snapshot usable and make every load failure retryable. */
export function SettingsLoadError({ error, retry, hasRows = false }: { error: unknown; retry: () => unknown; hasRows?: boolean }) {
  if (!error) return null;
  return <div role="alert" className="flex flex-wrap items-center justify-between gap-2 px-4 py-3">
    <p className="min-w-0 flex-1 text-body-sm text-danger">{hasRows ? "Refresh failed. Showing previously loaded rows." : "Couldn't load this collection."}</p>
    <Button size="sm" variant="secondary" onPress={() => { void retry(); }}>Retry</Button>
  </div>;
}
