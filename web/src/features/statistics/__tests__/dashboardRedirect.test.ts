import { describe, expect, it } from "vitest";

import { historyVariables, LOCAL_ORIGIN } from "../HistoryPage";
import { dashboardRedirect } from "../dashboardRedirect";

describe("old dashboard addresses", () => {
  it("open your profile on the matching tab", () => {
    expect(dashboardRedirect("/dashboard", {}, false)).toBe("/?profile=statistics");
    expect(dashboardRedirect("/dashboard/statistics", {}, false)).toBe("/?profile=statistics");
    expect(dashboardRedirect("/dashboard/statistics", { period: "7d" }, true)).toBe("/?profile=statistics");
    expect(dashboardRedirect("/dashboard/history", {}, true)).toBe("/?profile=history");
    expect(dashboardRedirect("/dashboard/devices", {}, false)).toBe("/?profile=devices");
    expect(dashboardRedirect("/dashboard/anything-else", {}, false)).toBe("/?profile=profile");
  });

  it("send an administrator's everyone views to Settings › Statistics", () => {
    expect(dashboardRedirect("/dashboard", {}, true)).toBe("/settings/statistics");
    expect(dashboardRedirect("/dashboard/", {}, true)).toBe("/settings/statistics");
    expect(dashboardRedirect("/dashboard/statistics", { user: "everyone", period: "90d" }, true)).toBe("/settings/statistics/overview?user=everyone&period=90d");
    expect(dashboardRedirect("/dashboard/statistics", { user: "amy", period: "bogus" }, true)).toBe("/settings/statistics/overview?user=amy");
  });

  it("never give a member the server page", () => {
    expect(dashboardRedirect("/dashboard/statistics", { user: "everyone" }, false)).toBe("/?profile=statistics");
  });
});

describe("the history's server filter", () => {
  const state = (values: string[]) => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, sorting: [], filters: [{ id: "origin", value: { values } }] }) as never;

  it("treats this server as plays without an origin", () => {
    expect(historyVariables(state([LOCAL_ORIGIN])).where).toEqual({ and: [{ origin: { isNull: true } }] });
    expect(historyVariables(state(["srv-2"])).where).toEqual({ and: [{ origin: { inList: ["srv-2"] } }] });
    expect(historyVariables(state([LOCAL_ORIGIN, "srv-2"])).where).toEqual({ and: [{ or: [{ origin: { isNull: true } }, { origin: { inList: ["srv-2"] } }] }] });
  });
});
