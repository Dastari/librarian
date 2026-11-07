import { afterEach, describe, expect, it, vi } from "vitest";

import { checkForUpdate, pwaStore, startPwa } from "../pwa";

describe("pwa outside production builds", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    Reflect.deleteProperty(navigator, "serviceWorker");
  });

  it("registers nothing, polls nothing and retires a worker left on the dev origin", async () => {
    const unregister = vi.fn(async () => true);
    const register = vi.fn();
    Object.defineProperty(navigator, "serviceWorker", {
      configurable: true,
      value: { register, getRegistrations: vi.fn(async () => [{ unregister }]) },
    });
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);
    const interval = vi.spyOn(window, "setInterval");

    startPwa();
    await vi.waitFor(() => expect(unregister).toHaveBeenCalled());
    expect(register).not.toHaveBeenCalled();
    expect(interval).not.toHaveBeenCalled();
    expect(await checkForUpdate()).toBe(false);
    expect(fetchSpy).not.toHaveBeenCalled();
    expect(pwaStore.getSnapshot()).toEqual({ updateAvailable: false, offlineReady: false });
  });
});
