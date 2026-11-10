import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { BeginTwoFactorSetupDocument, ConfirmTwoFactorSetupDocument, EntityUserGetDocument, GenreChoicesDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { qrImageSource, TwoFactorSetupDialog } from "../TwoFactorDialogs";
import { UserSettingsDialog } from "../UserSettingsDialog";

const auth = vi.hoisted(() => ({ admin: false, id: "me" }));
vi.mock("@/lib/auth/useSession", () => ({
  useIsAdmin: () => auth.admin,
  useSession: () => ({ status: "authenticated", user: { id: auth.id, role: auth.admin ? "admin" : "member", username: "toby", displayName: "Toby", email: null, avatarUrl: null } }),
}));
// The statistics, history and device views have their own tests.
vi.mock("@/features/statistics/StatisticsPage", () => ({ StatisticsPage: (props: { fixedPerson?: string }) => <p>Statistics view{props.fixedPerson ? ` for ${props.fixedPerson}` : ""}</p> }));
vi.mock("@/features/statistics/HistoryPage", () => ({ HistoryPage: (props: { userId?: string }) => <p>History of {props.userId}</p> }));
vi.mock("@/features/statistics/DevicesPage", () => ({ DevicesPage: (props: { userId?: string }) => <p>Devices of {props.userId}</p> }));

const account = (id: string, extra: Record<string, unknown> = {}) => ({
  __typename: "User", id, username: id === "me" ? "toby" : "amy", email: `${id}@example.test`, role: "member", displayName: id === "me" ? "Toby" : "Amy",
  avatarUrl: null, isActive: true, lastLoginAt: null, createdAt: "1700000000", updatedAt: "1700000000", phone: null, genreInterests: [],
  mustChangePassword: false, twoFactorEnabledAt: null, recoveryCodesRemaining: 0, passwordChangedAt: null, ...extra,
});
const userMock = (id: string, extra?: Record<string, unknown>) => ({ request: { query: EntityUserGetDocument, variables: { id } }, result: { data: { user: account(id, extra) } } });
const genresMock = { request: { query: GenreChoicesDocument }, result: { data: { genreChoices: [{ __typename: "GenreChoice", genre: "Sci-Fi", movies: 3, shows: 1 }] } } };

const tabs = () => screen.getAllByRole("tab").map((tab) => tab.textContent?.trim());

describe("the profile dialog", () => {
  beforeEach(() => {
    auth.admin = false;
    auth.id = "me";
  });

  it("has your profile, security, devices, preferences, statistics and history", async () => {
    await renderWithProviders(<UserSettingsDialog userId="me" open tab="profile" onTabChange={() => {}} onOpenChange={() => {}} />, { mocks: [userMock("me"), genresMock] });
    expect(await screen.findByLabelText("Display name")).toHaveValue("Toby");
    expect(tabs()).toEqual(["Profile", "Security", "Devices", "Preferences", "Statistics", "History"]);
    expect(screen.getByLabelText("Email")).toHaveValue("me@example.test");
  });

  it("shows two-factor sign-in as off with a way to turn it on", async () => {
    await renderWithProviders(<UserSettingsDialog userId="me" open tab="security" onTabChange={() => {}} onOpenChange={() => {}} />, { mocks: [userMock("me"), genresMock] });
    expect(await screen.findByText(/Off\. Signing in can also ask for a code/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Turn on…" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Change password…" })).toBeInTheDocument();
  });

  it("gives an administrator account recovery, not preferences, for someone else", async () => {
    auth.admin = true;
    await renderWithProviders(<UserSettingsDialog userId="amy" open tab="security" onTabChange={() => {}} onOpenChange={() => {}} />, { mocks: [userMock("amy", { twoFactorEnabledAt: "1700000000" })] });
    expect(await screen.findByRole("button", { name: "Temporary password…" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reset link…" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Turn off…" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Change password…" })).not.toBeInTheDocument();
    expect(tabs()).toEqual(["Profile", "Security", "Devices", "Statistics", "History"]);
  });

  it("shows someone else's statistics pinned to them", async () => {
    auth.admin = true;
    await renderWithProviders(<UserSettingsDialog userId="amy" open tab="statistics" onTabChange={() => {}} onOpenChange={() => {}} />, { mocks: [userMock("amy")] });
    expect(await screen.findByText("Statistics view for amy")).toBeInTheDocument();
  });
});

describe("turning on two-factor sign-in", () => {
  it("asks for the password, shows the QR code and key, confirms a code, then shows recovery codes", async () => {
    const svg = "<svg xmlns='http://www.w3.org/2000/svg'></svg>";
    const codes = ["abcde-fghjk", "mnpqr-stuvw"];
    const onChanged = vi.fn();
    await renderWithProviders(<TwoFactorSetupDialog open onOpenChange={() => {}} onChanged={onChanged} />, {
      mocks: [
        { request: { query: BeginTwoFactorSetupDocument, variables: { input: { password: "my-password" } } }, result: { data: { beginTwoFactorSetup: { __typename: "TwoFactorSetupResult", success: true, error: null, secret: "JBSWY3DPEHPK3PXP", uri: "otpauth://totp/Librarian:me", qrSvg: svg } } } },
        { request: { query: ConfirmTwoFactorSetupDocument, variables: { input: { code: "123456" } } }, result: { data: { confirmTwoFactorSetup: { __typename: "RecoveryCodesResult", success: true, error: null, recoveryCodes: codes } } } },
      ],
    });
    await userEvent.type(screen.getByLabelText("Your password"), "my-password");
    await userEvent.click(screen.getByRole("button", { name: "Continue" }));
    const qr = await screen.findByRole("img", { name: "QR code for your authenticator app" });
    expect(qr).toHaveAttribute("src", qrImageSource(svg));
    expect(screen.getByText("JBSW Y3DP EHPK 3PXP")).toBeInTheDocument();
    await userEvent.click(screen.getAllByRole("textbox")[0]!);
    // A complete code submits by itself.
    await userEvent.keyboard("123456");
    expect(await screen.findByRole("list", { name: "Recovery codes" })).toHaveTextContent("abcde-fghjk");
    expect(onChanged).toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "I've saved them" })).toBeInTheDocument();
  });
});
