import { screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { RouteError } from "../RouteError";

const reloadOnceForStaleChunk = vi.fn(() => true);
// `@/main` boots the app when it is imported; the reload guard is all this component needs.
vi.mock("@/main", () => ({ reloadOnceForStaleChunk: () => reloadOnceForStaleChunk() }));

type RouteErrorProps = ComponentProps<typeof RouteError>;
/** TanStack hands an error component far more than it reads; only these fields matter here. */
const props = (error: Error, reset = vi.fn()) => ({ error, reset, info: { componentStack: "" }, params: {}, search: {} }) as unknown as RouteErrorProps;

beforeEach(() => reloadOnceForStaleChunk.mockClear());

describe("RouteError", () => {
  it("shows the failure as an alert with the server's message", async () => {
    await renderWithProviders(<RouteError {...props(new Error("Library not found"))} />);
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Something went wrong" })).toBeInTheDocument();
    expect(screen.getByText("Library not found")).toBeInTheDocument();
    expect(reloadOnceForStaleChunk).not.toHaveBeenCalled();
  });

  it("retries through the router", async () => {
    const reset = vi.fn();
    const { router } = await renderWithProviders(<RouteError {...props(new Error("boom"), reset)} />);
    const invalidate = vi.spyOn(router!, "invalidate").mockResolvedValue(undefined);
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(reset).toHaveBeenCalledOnce();
    expect(invalidate).toHaveBeenCalledOnce();
  });

  it("goes home", async () => {
    const { router } = await renderWithProviders(<RouteError {...props(new Error("boom"))} />, { path: "/movies" });
    await userEvent.click(screen.getByRole("button", { name: "Go home" }));
    await vi.waitFor(() => expect(router!.state.location.pathname).toBe("/"));
  });

  it("reloads once for a stale chunk after a deploy", async () => {
    await renderWithProviders(<RouteError {...props(new Error("Failed to fetch dynamically imported module: /assets/movies-abc.js"))} />);
    expect(reloadOnceForStaleChunk).toHaveBeenCalledOnce();
    expect(screen.getByRole("heading", { name: "Reloading the app" })).toBeInTheDocument();
    expect(screen.getByText(/newer version of Librarian/)).toBeInTheDocument();
  });

  it("recognises the other stale-chunk wordings browsers use", async () => {
    for (const message of ["Importing a module script failed.", "error loading dynamically imported module: /assets/x.js"]) {
      const { unmount } = await renderWithProviders(<RouteError {...props(new Error(message))} />);
      expect(screen.getByRole("heading", { name: "Reloading the app" })).toBeInTheDocument();
      unmount();
    }
    expect(reloadOnceForStaleChunk).toHaveBeenCalledTimes(2);
  });
});
