import { describe, expect, it } from "vitest";

import { feedIntervalOptions, iptorrentsCredentials } from "../SourceDialog";

describe("iptorrentsCredentials", () => {
  const empty = { username: "", password: "", cookie: "", userAgent: "" };

  it("sends a username and password by default, with the method named", () => {
    expect(iptorrentsCredentials("password", { ...empty, username: " someone ", password: "p@ss word" })).toEqual({ AuthMethod: "password", Username: "someone", Password: "p@ss word" });
  });

  it("sends the cookie and user agent for the advanced method, and only the agent that was given", () => {
    expect(iptorrentsCredentials("cookie", { ...empty, cookie: " uid=1; pass=2 ", userAgent: "Mozilla/5.0" })).toEqual({ AuthMethod: "cookie", Cookie: "uid=1; pass=2", UserAgent: "Mozilla/5.0" });
    expect(iptorrentsCredentials("cookie", { ...empty, cookie: "uid=1; pass=2" })).toEqual({ AuthMethod: "cookie", Cookie: "uid=1; pass=2" });
  });

  it("sends nothing when the fields for the chosen method are empty, so a saved login is kept", () => {
    expect(iptorrentsCredentials("password", { ...empty, username: "someone" })).toBeNull();
    expect(iptorrentsCredentials("password", { ...empty, cookie: "uid=1; pass=2" })).toBeNull();
    expect(iptorrentsCredentials("cookie", { ...empty, username: "someone", password: "x" })).toBeNull();
  });
});

describe("feedIntervalOptions", () => {
  it("offers 15, 30 and 60 minutes, and a feed's own interval if it is something else", () => {
    expect(feedIntervalOptions(undefined)).toEqual([15, 30, 60]);
    expect(feedIntervalOptions(30)).toEqual([15, 30, 60]);
    expect(feedIntervalOptions(45)).toEqual([15, 30, 45, 60]);
  });
});
