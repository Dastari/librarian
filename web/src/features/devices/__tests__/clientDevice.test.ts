import { afterEach, describe, expect, it, vi } from "vitest";

import { clientCheckIn, clientDeviceId, currentDevice, defaultDeviceName, describeClient } from "../clientDevice";
import { deviceState } from "../DeviceIdentity";

const UA = {
  ipad: "Mozilla/5.0 (iPad; CPU OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
  // iPadOS 13+ asks for the desktop site and says Macintosh; only touch gives it away.
  ipadDesktop: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15",
  iphone: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
  pixel: "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36",
  galaxyTab: "Mozilla/5.0 (Linux; Android 13; SM-X700) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
  lg: "Mozilla/5.0 (Web0S; Linux/SmartTV) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/94.0 Safari/537.36 WebAppManager",
  samsung: "Mozilla/5.0 (SMART-TV; LINUX; Tizen 6.5) AppleWebKit/537.36 (KHTML, like Gecko) 85.0 TV Safari/537.36",
  fire: "Mozilla/5.0 (Linux; Android 9; AFTMM Build/PS7633) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Mobile Safari/537.36",
  edge: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36 Edg/124.0",
  firefoxMac: "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.4; rv:125.0) Gecko/20100101 Firefox/125.0",
};

describe("identifying this browser as a device", () => {
  it("names phones, tablets and computers by browser and platform", () => {
    expect(defaultDeviceName(describeClient(UA.ipad))).toBe("Safari on iPad");
    expect(defaultDeviceName(describeClient(UA.ipadDesktop, 5))).toBe("Safari on iPad");
    expect(describeClient(UA.ipadDesktop, 5).formFactor).toBe("tablet");
    expect(defaultDeviceName(describeClient(UA.ipadDesktop, 0))).toBe("Safari on Mac");
    expect(defaultDeviceName(describeClient(UA.iphone))).toBe("Safari on iPhone");
    expect(describeClient(UA.iphone).formFactor).toBe("phone");
    expect(describeClient(UA.pixel)).toEqual({ product: "Chrome", platform: "Android", formFactor: "phone" });
    expect(describeClient(UA.galaxyTab).formFactor).toBe("tablet");
    expect(defaultDeviceName(describeClient(UA.edge))).toBe("Edge on Windows");
    expect(defaultDeviceName(describeClient(UA.firefoxMac))).toBe("Firefox on Mac");
    expect(defaultDeviceName(describeClient(""))).toBe("Unknown device");
  });

  it("recognises TVs and calls them TVs", () => {
    expect(describeClient(UA.lg)).toMatchObject({ platform: "webOS", formFactor: "tv" });
    expect(defaultDeviceName(describeClient(UA.lg))).toBe("webOS TV");
    expect(defaultDeviceName(describeClient(UA.samsung))).toBe("Tizen TV");
    expect(defaultDeviceName(describeClient(UA.fire))).toBe("Fire TV");
  });

  it("keeps one id per browser", () => {
    localStorage.removeItem("librarian.device.v1");
    const first = clientDeviceId();
    expect(first).toMatch(/^[A-Za-z0-9_-]{1,128}$/);
    expect(clientDeviceId()).toBe(first);
    localStorage.setItem("librarian.device.v1", "not valid!");
    expect(clientDeviceId()).not.toBe("not valid!");
  });

  it("checks in as a TV in TV mode, whatever the browser", () => {
    const tv = clientCheckIn({ tvMode: true, appVersion: "0.6.0" });
    expect(tv.formFactor).toBe("tv");
    expect(tv.name).toBe("TV");
    expect(tv.appVersion).toBe("0.6.0");
    expect(clientCheckIn({ tvMode: false }).formFactor).not.toBe("tv");
  });

  it("tells listeners which device row it checked in as", () => {
    const listener = vi.fn();
    const stop = currentDevice.subscribe(listener);
    currentDevice.set("device-1");
    currentDevice.set("device-1");
    expect(listener).toHaveBeenCalledTimes(1);
    expect(currentDevice.get()).toBe("device-1");
    stop();
    currentDevice.set(null);
  });
});

describe("a device's state", () => {
  afterEach(() => vi.useRealTimers());
  const now = Date.parse("2026-11-09T12:00:00Z");
  const device = { id: "d", userId: "u", name: "TV", formFactor: "tv", lastSeenAt: "2026-11-09T11:55:00.000Z", signedOutAt: null };

  it("marks the device in use, active ones, idle ones and signed-out ones", () => {
    expect(deviceState(device, { current: true, now }).label).toBe("This device");
    expect(deviceState(device, { now })).toMatchObject({ label: "Active now", tone: "success" });
    expect(deviceState({ ...device, lastSeenAt: "2026-11-08T12:00:00.000Z" }, { now }).label).toMatch(/^Seen /);
    expect(deviceState({ ...device, signedOutAt: "2026-11-09T11:59:00.000Z" }, { now }).label).toBe("Signed out");
  });
});
