import { isPeriod } from "./statistics";

/**
 * Where an old `/dashboard…` address goes now that the dashboard is gone (design.md Q147, Q150):
 * your own views open your profile on that tab, and administrators' everyone views go to
 * Settings › Statistics. Returns an `href`.
 */
export function dashboardRedirect(pathname: string, search: Record<string, unknown>, isAdmin: boolean): string {
  const view = pathname.replace(/^\/dashboard\/?/, "").split("/")[0] ?? "";
  const period = isPeriod(search.period) ? search.period : undefined;
  const user = typeof search.user === "string" && search.user ? search.user : undefined;
  const query = (entries: Record<string, string | undefined>) => {
    const params = new URLSearchParams(Object.entries(entries).filter((entry): entry is [string, string] => Boolean(entry[1])));
    const text = params.toString();
    return text ? `?${text}` : "";
  };
  switch (view) {
    case "":
      // Now playing for everyone was an administrator's view; yours is the player itself.
      return isAdmin ? "/settings/statistics" : "/?profile=statistics";
    case "statistics":
      if (isAdmin && user) return `/settings/statistics/overview${query({ user, period })}`;
      return `/${query({ profile: "statistics" })}`;
    case "history":
      return "/?profile=history";
    case "devices":
      return "/?profile=devices";
    default:
      return "/?profile=profile";
  }
}
