import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { Segmented } from "glass-ux/segmented";

import { GuideCalendar } from "./GuideCalendar";
import { StreamingGrid } from "./StreamingGrid";

type View = "calendar" | "grid";
const route = getRouteApi("/_app/guide/");

/**
 * The full TV guide: your shows on a calendar, or the week's listings as a services × days
 * grid. Both sit on the canvas under the same toolbar. The view lives in the URL so either one
 * can be linked and restored.
 */
export function GuidePage() {
  const navigate = useNavigate();
  const { view = "calendar" } = route.useSearch();
  const setView = (next: View) => void navigate({ to: "/guide", search: next === "calendar" ? {} : { view: next }, replace: true });
  const switcher = <Segmented<View> aria-label="Guide view" value={view} onValueChange={setView} options={[{ value: "calendar", label: "Calendar" }, { value: "grid", label: "Streaming grid" }]} data-tv-autofocus />;
  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 pb-4 pt-1">
      {view === "grid" ? <StreamingGrid switcher={switcher} /> : <GuideCalendar switcher={switcher} />}
    </div>
  );
}
