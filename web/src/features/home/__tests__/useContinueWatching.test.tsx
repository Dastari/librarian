import { MockedProvider } from "@apollo/client/testing/react";
import { renderHook, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { ResumeShelfDocument } from "@/graphql/generated/graphql";
import { useContinueWatching } from "../useContinueWatching";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "me", role: "member" }, status: "authenticated" }) }));
it("exposes an unproven resume window and its budget error without fabricating an exhaustive empty shelf", async () => {
  const wrapper = ({ children }: { children: ReactNode }) => <MockedProvider mocks={[{
    request: { query: ResumeShelfDocument, variables: () => true },
    result: { data: { resumeShelf: { items: [], hasMore: null, complete: false, scannedCount: 10_000, error: "Resume history exceeded its proof budget" } } },
  }]}>{children}</MockedProvider>;
  const { result } = renderHook(() => useContinueWatching(), { wrapper });
  await waitFor(() => expect(result.current.loading).toBe(false));
  expect(result.current.items).toEqual([]);
  expect(result.current.complete).toBe(false);
  expect(result.current.hasMore).toBeNull();
  expect(result.current.scannedCount).toBe(10_000);
  expect(result.current.error?.message).toBe("Resume history exceeded its proof budget");
});
