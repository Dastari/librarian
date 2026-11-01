import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";
import { UsersSettings } from "../UsersSettings";

const state = vi.hoisted(() => ({ update: vi.fn(), remove: vi.fn(), revoke: vi.fn(), refresh: vi.fn() }));
vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "me", role: "admin" } }) }));
vi.mock("../useSettingsConnection", async () => {
  const docs = await import("@/graphql/generated/graphql");
  return { useSettingsConnection: (document: unknown) => ({
    rows: document === docs.EntityUserListDocument ? [
      { id: "me", username: "administrator", displayName: "Me", email: null, role: "admin", isActive: true, lastLoginAt: null, createdAt: "2026-10-31T00:00:00Z" },
      { id: "member-a", username: "member_a", displayName: "Member A", email: null, role: "member", isActive: true, lastLoginAt: null, createdAt: "2026-10-31T00:00:00Z" },
      { id: "member-b", username: "member_b", displayName: "Member B", email: null, role: "member", isActive: true, lastLoginAt: null, createdAt: "2026-10-31T00:00:00Z" },
    ] : document === docs.EntityInviteTokenListDocument ? [
      { id: "invite-a", role: "member", useCount: 0, maxUses: 1, expiresAt: null },
      { id: "invite-b", role: "member", useCount: 0, maxUses: 1, expiresAt: null },
    ] : [], loading: false, error: undefined, refetch: state.refresh,
  }) };
});
vi.mock("@apollo/client/react", async (original) => {
  const docs = await import("@/graphql/generated/graphql");
  return { ...await original<typeof import("@apollo/client/react")>(), useMutation: (document: unknown) => [
    document === docs.EntityUserUpdateDocument ? state.update : document === docs.EntityUserDeleteDocument ? state.remove : document === docs.EntityInviteTokenDeleteDocument ? state.revoke : vi.fn(), { loading: false },
  ] };
});

beforeEach(() => {
  vi.clearAllMocks();
  state.update.mockResolvedValue({ data: { updateUser: { success: true } } });
  state.remove.mockResolvedValue({ data: { deleteUser: { success: true } } });
  state.revoke.mockResolvedValue({ data: { deleteInviteToken: { success: true } } });
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ x: 0, y: 0, top: 0, left: 0, bottom: 800, right: 1440, width: 1440, height: 800, toJSON: () => ({}) });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(1440);
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1440);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
});
afterEach(() => vi.restoreAllMocks());

describe("user and invite selection actions", () => {
  it("keeps self-destructive and self-role changes disabled in a mixed selection", async () => {
    await renderWithProviders(<UsersSettings />, { router: false });
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Me" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Member A" }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected users" }));
    expect(await screen.findByRole("menuitem", { name: "Delete 2 users" })).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByRole("menuitem", { name: "Disable sign-in" })).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByRole("menuitem", { name: "Make admins" })).toHaveAttribute("aria-disabled", "true");
    expect(state.update).not.toHaveBeenCalled();
    expect(state.remove).not.toHaveBeenCalled();
  });

  it("confirms a batch once and disables only the selected users", async () => {
    await renderWithProviders(<UsersSettings />, { router: false });
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Member A" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Member B" }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected users" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Disable sign-in" }));
    const confirm = await screen.findByRole("alertdialog");
    expect(state.update).not.toHaveBeenCalled();
    await userEvent.click(within(confirm).getByRole("button", { name: "Disable" }));
    await waitFor(() => expect(state.update).toHaveBeenCalledTimes(2));
    expect(state.update).toHaveBeenCalledWith({ variables: { id: "member-a", input: { isActive: false } } });
    expect(state.update).toHaveBeenCalledWith({ variables: { id: "member-b", input: { isActive: false } } });
    expect(state.refresh).toHaveBeenCalledOnce();
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });

  it("revokes both selected invites through checked generated mutations", async () => {
    await renderWithProviders(<UsersSettings />, { router: false });
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Invite invite-a" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Invite invite-b" }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected invites" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Revoke 2 invites" }));
    const confirm = await screen.findByRole("alertdialog");
    expect(state.revoke).not.toHaveBeenCalled();
    await userEvent.click(within(confirm).getByRole("button", { name: "Revoke" }));
    await waitFor(() => expect(state.revoke).toHaveBeenCalledTimes(2));
    expect(state.revoke).toHaveBeenCalledWith({ variables: { id: "invite-a" } });
    expect(state.revoke).toHaveBeenCalledWith({ variables: { id: "invite-b" } });
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });
});
