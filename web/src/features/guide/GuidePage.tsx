import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { Segmented } from "glass-ux/segmented";

import { PageScroller } from "@/components/shell/PageScroller";

import { GuideCalendar } from "./GuideCalendar";
import { StreamingGrid } from "./StreamingGrid";

type View = "calendar" | "grid";
const route = getRouteApi("/_app/guide/");

/**
 * The full TV guide: your shows on a calendar, or the week's listings as a services × days
 * grid. The view lives in the URL so either one can be linked and restored.
 */
export function GuidePage() {
  const navigate = useNavigate();
  const { view = "calendar" } = route.useSearch();
  const setView = (next: View) => void navigate({ to: "/guide", search: next === "calendar" ? {} : { view: next }, replace: true });
  const switcher = <Segmented<View> aria-label="Guide view" value={view} onValueChange={setView} options={[{ value: "calendar", label: "Calendar" }, { value: "grid", label: "Streaming grid" }]} data-tv-autofocus />;
  // The grid holds the switch in its header and scrolls its own rows; the calendar scrolls under it.
  if (view === "grid") return <div className="page-gutter flex min-h-0 min-w-0 flex-1 flex-col pb-4 pt-1"><StreamingGrid switcher={switcher} /></div>;
  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 pt-1">
      <div className="page-gutter flex shrink-0 flex-wrap items-center gap-3" data-nav-section>
        <p className="min-w-0 flex-1 text-body-sm text-muted">Episodes of the shows in your libraries.</p>
        {switcher}
      </div>
      <PageScroller className="page-gutter flex-1 pb-6"><GuideCalendar /></PageScroller>
    </div>
  );
}
