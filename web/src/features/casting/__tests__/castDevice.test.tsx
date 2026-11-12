import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { CastDeviceFieldsFragment } from "@/graphql/generated/graphql";

import { castDeviceForm, castDeviceState, makeAndModel, playbackBlocker, protocolLabel, reportedDisplayName } from "../castDevice";
import { CastDeviceDiscovery } from "../CastDeviceSettingsDialog";

const STAMP = "2026-11-12T03:00:00Z";

function device(overrides: Partial<CastDeviceFieldsFragment> = {}): CastDeviceFieldsFragment {
  return {
    id: "d1", name: "Samsung 8 Series (55)", address: "10.0.1.223", port: 9197, model: "UA55KS8000", deviceType: "DLNA_RENDERER",
    enabled: true, playbackSupported: false, isFavorite: false, isManual: false, discoveryOrigin: "NETWORK",
    firstSeenAt: STAMP, lastSeenAt: STAMP, lastProbeAt: STAMP, lastProbeError: null, createdAt: STAMP, updatedAt: STAMP,
    manufacturer: "Samsung Electronics", receiverId: "uuid:a32ea4b3", nameIsCustom: false,
    discoveryDetails: JSON.stringify({
      protocol: "dlna", reportedName: "[TV] Samsung 8 Series (55)", modelDescription: "Samsung TV DMR",
      descriptionUrl: "http://10.0.1.223:9197/dmr", server: "SHP, UPnP/1.0, Samsung UPnP SDK/1.0",
      usn: "uuid:a32ea4b3::urn:schemas-upnp-org:device:MediaRenderer:1", services: ["RenderingControl:1", "AVTransport:1"],
    }),
    ...overrides,
  };
}

describe("cast device helpers", () => {
  it("names, classifies and explains a Samsung DLNA renderer", () => {
    const samsung = device();
    expect(reportedDisplayName(samsung)).toBe("Samsung 8 Series (55)");
    expect(makeAndModel(samsung)).toBe("Samsung Electronics UA55KS8000");
    expect(protocolLabel(samsung)).toBe("DLNA");
    expect(castDeviceForm(samsung)).toBe("tv");
    expect(playbackBlocker(samsung)).toMatch(/DLNA/);
  });

  it("treats Google Cast receivers as playable and audio-only ones as speakers", () => {
    const tv = device({ deviceType: "CHROMECAST", playbackSupported: true, manufacturer: null, model: "Google TV Streamer", discoveryDetails: null });
    expect(playbackBlocker(tv)).toBeUndefined();
    expect(makeAndModel(tv)).toBe("Google TV Streamer");
    expect(castDeviceForm(tv)).toBe("tv");
    const speaker = device({ deviceType: "CHROMECAST_AUDIO", playbackSupported: true, model: "Nest Audio", discoveryDetails: null });
    expect(protocolLabel(speaker)).toBe("Google Cast · audio only");
    expect(castDeviceForm(speaker)).toBe("speaker");
    expect(playbackBlocker(device({ deviceType: "CHROMECAST", playbackSupported: true, enabled: false }))).toMatch(/Turned off/);
  });

  it("reads state from what discovery last saw", () => {
    const now = Date.parse(STAMP) + 60_000;
    expect(castDeviceState(device(), now).id).toBe("available");
    expect(castDeviceState(device({ lastSeenAt: "2026-11-01T00:00:00Z" }), now).id).toBe("away");
    expect(castDeviceState(device({ lastProbeError: "timed out" }), now).id).toBe("error");
    expect(castDeviceState(device({ enabled: false }), now).id).toBe("disabled");
    expect(castDeviceState(device({ lastSeenAt: null, isManual: true }), now).label).toBe("Added by address");
  });

  it("survives details that are missing or not JSON", () => {
    expect(reportedDisplayName(device({ discoveryDetails: null }))).toBeUndefined();
    expect(reportedDisplayName(device({ discoveryDetails: "{nope" }))).toBeUndefined();
  });
});

describe("CastDeviceDiscovery", () => {
  it("shows how a DLNA renderer was found", () => {
    render(<CastDeviceDiscovery device={device()} />);
    expect(screen.getByText("Found on the network (SSDP)")).toBeInTheDocument();
    expect(screen.getByText("[TV] Samsung 8 Series (55)")).toBeInTheDocument();
    expect(screen.getByText("http://10.0.1.223:9197/dmr")).toBeInTheDocument();
    expect(screen.getByText("RenderingControl:1, AVTransport:1")).toBeInTheDocument();
    expect(screen.getByText("10.0.1.223:9197")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "DLNA (SSDP and UPnP)" })).toBeInTheDocument();
  });

  it("shows a Google Cast receiver's TXT record", () => {
    render(<CastDeviceDiscovery device={device({
      deviceType: "CHROMECAST", playbackSupported: true, manufacturer: null, model: "Google TV Streamer",
      discoveryDetails: JSON.stringify({ protocol: "googleCast", reportedName: "Lounge", serviceName: "Google-TV-Streamer-abc._googlecast._tcp.local.", capabilities: ["Video out", "Audio out"], txt: { fn: "Lounge", md: "Google TV Streamer", ca: "5" } }),
    })} />);
    expect(screen.getByText("Found on the network (mDNS)")).toBeInTheDocument();
    expect(screen.getByText("Video out, Audio out")).toBeInTheDocument();
    expect(screen.getByText("TXT record")).toBeInTheDocument();
    expect(screen.getByText("Google-TV-Streamer-abc._googlecast._tcp.local.")).toBeInTheDocument();
    expect(screen.getByText("Yes")).toBeInTheDocument();
  });
});
