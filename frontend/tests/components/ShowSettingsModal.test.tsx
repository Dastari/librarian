// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
vi.mock("@tanstack/react-router", () => ({ Link: ({ children }: { children: React.ReactNode }) => <span>{children}</span> }));
vi.mock("../../src/components/library/QualityProfileSelector", () => ({
  QualityProfileSelector: ({ value, onChange }: { value: string | null; onChange: (value: string | null) => void }) => (
    <select aria-label="Quality Profile" value={value ?? ""} onChange={e => onChange(e.target.value || null)}>
      <option value="">Inherit</option><option value="hd">1080p</option>
    </select>
  ),
}));
import { ShowSettingsModal } from "../../src/components/shows/ShowSettingsModal";
afterEach(cleanup);
it("saves supported show settings, including clearing its quality override", async () => {
  const save = vi.fn().mockResolvedValue(undefined);
  render(<ShowSettingsModal isOpen onClose={() => {}} isLoading={false} onSave={save}
    show={{ name: "Percy Jackson", autoDownload: true, autoDownloadMode: "WANTED", qualityProfileId: "hd" }} />);
  expect(screen.getByText("Percy Jackson — Show Settings")).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Quality Profile"), { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "Save Settings" }));
  await waitFor(() => expect(save).toHaveBeenCalledWith({ autoDownload: true, autoDownloadMode: "WANTED", qualityProfileId: null }));
});
