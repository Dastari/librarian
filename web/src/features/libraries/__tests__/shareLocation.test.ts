import { describe, expect, it } from "vitest";

import { composeShareAddress, isShareAddress, parseShareAddress, shareRootAddress, splitHostPort } from "../shareLocation";

describe("share addresses", () => {
  it("splits an address into server, share and folder", () => {
    expect(parseShareAddress("smb://10.0.1.190/media/Movies/Kids")).toEqual({ host: "10.0.1.190", share: "media", folder: "Movies/Kids" });
    expect(parseShareAddress("smb://truenas.localdomain/media")).toEqual({ host: "truenas.localdomain", share: "media", folder: "" });
    expect(parseShareAddress("smb://nas:1445/media/")).toEqual({ host: "nas:1445", share: "media", folder: "" });
    expect(parseShareAddress("smb://nas")).toEqual({ host: "nas", share: "", folder: "" });
    expect(parseShareAddress("/data/media")).toBeNull();
  });

  it("joins the parts into the address the server keeps", () => {
    expect(composeShareAddress({ host: " nas ", share: "/media/", folder: "/Movies//Kids/" })).toBe("smb://nas/media/Movies/Kids");
    expect(composeShareAddress({ host: "nas", share: "media", folder: "" })).toBe("smb://nas/media");
  });

  it("is empty until the server and the share are both given", () => {
    expect(composeShareAddress({ host: "", share: "media", folder: "Movies" })).toBe("");
    expect(composeShareAddress({ host: "nas", share: "", folder: "Movies" })).toBe("");
  });

  it("round-trips folders with spaces and punctuation", () => {
    const location = { host: "nas", share: "media", folder: "Audio Books/Guns N’ Roses (1991)" };
    expect(parseShareAddress(composeShareAddress(location))).toEqual(location);
  });

  it("recognises a share address in any case", () => {
    expect(isShareAddress("SMB://nas/media")).toBe(true);
    expect(isShareAddress(" smb://nas/media")).toBe(true);
    expect(isShareAddress("/smb://nas")).toBe(false);
  });

  it("takes an optional port after the server name", () => {
    expect(splitHostPort("10.0.1.190")).toEqual({ host: "10.0.1.190", port: null });
    expect(splitHostPort(" nas:1445 ")).toEqual({ host: "nas", port: 1445 });
    expect(splitHostPort("truenas.localdomain")).toEqual({ host: "truenas.localdomain", port: null });
  });

  it("names a share by its server and share only", () => {
    expect(shareRootAddress({ host: "nas", share: "media" })).toBe("smb://nas/media");
  });
});
