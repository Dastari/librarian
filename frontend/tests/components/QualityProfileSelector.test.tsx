// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
const profileId = "d95c81c8-b5fe-4fb4-89ca-6f2cb80d2b34";
vi.mock("../../src/lib/graphql/client", () => ({ apolloClient: { query: async () => ({ data: { qualityProfiles: { edges: [{ node: { id: "d95c81c8-b5fe-4fb4-89ca-6f2cb80d2b34", name: "1080p" } }] } } }) } }));
import { QualityProfileSelector } from "../../src/components/library/QualityProfileSelector";
afterEach(cleanup);
it("keeps the entire profile ID when selection comes from the native select", async () => {
  const change = vi.fn();
  const { container } = render(<QualityProfileSelector value={null} onChange={change} allowInherit />);
  await waitFor(() => expect(container.querySelector("select")).toBeTruthy());
  fireEvent.change(container.querySelector("select")!, { target: { value: profileId } });
  await waitFor(() => expect(change).toHaveBeenLastCalledWith(profileId));
  expect(change).not.toHaveBeenCalledWith("d");
});
