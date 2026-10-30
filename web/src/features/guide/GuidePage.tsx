import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { Segmented } from "glass-ux/segmented";

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
  return (
    <div className="page-gutter flex min-w-0 flex-col gap-5 py-4 md:gap-6 md:py-6">
      <div className="flex flex-wrap items-center gap-3" data-nav-section>
        <p className="min-w-0 flex-1 text-body-sm text-muted">{view === "calendar" ? "Episodes of the shows in your libraries." : "Everything releasing this week, by service."}</p>
        <Segmented<View> aria-label="Guide view" value={view} onValueChange={setView} options={[{ value: "calendar", label: "Calendar" }, { value: "grid", label: "Streaming grid" }]} data-tv-autofocus />
      </div>
      {view === "calendar" ? <GuideCalendar /> : <StreamingGrid />}
    </div>
  );
}
