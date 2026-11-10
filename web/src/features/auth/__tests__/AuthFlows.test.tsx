import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CompleteTwoFactorSignInDocument, LoginDocument, RegisterDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { renderWithProviders } from "@/test";

import { LoginForm } from "../LoginForm";
import { RegisterForm } from "../RegisterForm";

const user = { __typename: "AuthenticatedUser", id: "u-1", email: "reader@example.com", username: "reader", role: "user", displayName: "Reader" };
const tokens = { __typename: "AuthSessionInfo", expiresIn: 3600, tokenType: "Bearer" };

afterEach(() => vi.restoreAllMocks());

async function fillAccount(password = "aaaaaaaaaa") {
  await userEvent.type(screen.getByLabelText("Name", { exact: true }), "Reader");
  await userEvent.type(screen.getByLabelText("Email", { exact: true }), "reader@example.com");
  await userEvent.type(screen.getByLabelText("Password", { exact: true }), password);
  await userEvent.type(screen.getByLabelText("Confirm password", { exact: true }), password);
}

describe("Glass authentication flows", () => {
  it("maps username sign-in to the cookie session and preserves the local redirect", async () => {
    const establish = vi.spyOn(session, "establish").mockImplementation(() => {});
    const { router } = await renderWithProviders(<LoginForm allowRegister redirectTo="/search?q=movie" />, {
      path: "/login",
      routes: ["/register", "/search"],
      mocks: [{
        request: { query: LoginDocument, variables: { input: { usernameOrEmail: "reader", password: "existing" } } },
        result: { data: { login: { __typename: "AuthPayload", success: true, error: null, user, tokens, challenge: null } } },
      }],
    });
    expect(screen.queryByText("Forgot password?")).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Create an account" })).toHaveAttribute("href", "/register");
    await userEvent.type(screen.getByLabelText("Username or email"), "reader");
    await userEvent.type(screen.getByLabelText("Password", { exact: true }), "existing");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    await vi.waitFor(() => expect(establish).toHaveBeenCalledWith(user, 3600));
    await vi.waitFor(() => expect(router?.state.location.href).toBe("/search?q=movie"));
  });

  it("keeps refused sign-in in place and does not establish a session", async () => {
    const establish = vi.spyOn(session, "establish").mockImplementation(() => {});
    await renderWithProviders(<LoginForm allowRegister={false} />, {
      mocks: [{
        request: { query: LoginDocument, variables: { input: { usernameOrEmail: "reader", password: "wrong" } } },
        result: { data: { login: { __typename: "AuthPayload", success: false, error: "Incorrect credentials", user: null, tokens: null, challenge: null } } },
      }],
    });
    await userEvent.type(screen.getByLabelText("Username or email"), "reader");
    await userEvent.type(screen.getByLabelText("Password", { exact: true }), "wrong");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByText("Incorrect credentials")).toBeInTheDocument();
    expect(establish).not.toHaveBeenCalled();
    expect(screen.queryByRole("link", { name: "Create an account" })).not.toBeInTheDocument();
  });

  it("asks for a code when two-factor sign-in is on, and signs in only once it is right", async () => {
    const establish = vi.spyOn(session, "establish").mockImplementation(() => {});
    const challenge = { __typename: "SignInChallenge", token: "opaque-challenge", stage: "two_factor", expiresIn: 300 };
    await renderWithProviders(<LoginForm allowRegister={false} />, {
      mocks: [
        { request: { query: LoginDocument, variables: { input: { usernameOrEmail: "reader", password: "existing" } } }, result: { data: { login: { __typename: "AuthPayload", success: false, error: null, user: null, tokens: null, challenge } } } },
        { request: { query: CompleteTwoFactorSignInDocument, variables: { input: { token: "opaque-challenge", code: "111111" } } }, result: { data: { completeTwoFactorSignIn: { __typename: "AuthPayload", success: false, error: "That code didn't work", user: null, tokens: null, challenge: null } } } },
        { request: { query: CompleteTwoFactorSignInDocument, variables: { input: { token: "opaque-challenge", code: "ABCDEFGHJK" } } }, result: { data: { completeTwoFactorSignIn: { __typename: "AuthPayload", success: true, error: null, user, tokens, challenge: null } } } },
      ],
    });
    await userEvent.type(screen.getByLabelText("Username or email"), "reader");
    await userEvent.type(screen.getByLabelText("Password", { exact: true }), "existing");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByRole("heading", { name: "Two-factor sign-in" })).toBeInTheDocument();
    // The password alone signed nobody in.
    expect(establish).not.toHaveBeenCalled();
    await userEvent.click(screen.getAllByRole("textbox")[0]!);
    await userEvent.keyboard("111111");
    expect(await screen.findByText("That code didn't work")).toBeInTheDocument();
    expect(establish).not.toHaveBeenCalled();
    // A recovery code instead (the field writes it in capitals; the server ignores case).
    await userEvent.click(screen.getByRole("button", { name: "Use a recovery code" }));
    await userEvent.click(screen.getAllByRole("textbox")[0]!);
    await userEvent.keyboard("abcdefghjk{Enter}");
    await vi.waitFor(() => expect(establish).toHaveBeenCalledWith(user, 3600));
  });

  it("asks for a new password after a temporary one", async () => {
    const establish = vi.spyOn(session, "establish").mockImplementation(() => {});
    await renderWithProviders(<LoginForm allowRegister={false} />, {
      mocks: [
        { request: { query: LoginDocument, variables: { input: { usernameOrEmail: "reader", password: "temporary" } } }, result: { data: { login: { __typename: "AuthPayload", success: false, error: null, user: null, tokens: null, challenge: { __typename: "SignInChallenge", token: "t", stage: "password_change", expiresIn: 300 } } } } },
      ],
    });
    await userEvent.type(screen.getByLabelText("Username or email"), "reader");
    await userEvent.type(screen.getByLabelText("Password", { exact: true }), "temporary");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByRole("heading", { name: "Choose a new password" })).toBeInTheDocument();
    expect(establish).not.toHaveBeenCalled();
  });

  it("collects the invitation before signup and accepts the existing ten-character password policy", async () => {
    const establish = vi.spyOn(session, "establish").mockImplementation(() => {});
    const register = vi.fn(() => ({ data: { register: { success: true, error: null, user, tokens } } }));
    await renderWithProviders(<RegisterForm isSetup={false} />, {
      path: "/register",
      routes: ["/login"],
      mocks: [{
        request: { query: RegisterDocument, variables: { input: { name: "Reader", email: "reader@example.com", password: "aaaaaaaaaa", inviteToken: "invite-1" } } },
        result: register,
      }],
    });
    expect(screen.queryByLabelText("Password", { exact: true })).not.toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("Invite code"), " invite-1 ");
    await userEvent.click(screen.getByRole("button", { name: "Continue" }));
    await fillAccount();
    await userEvent.click(screen.getByRole("button", { name: "Create account" }));
    await vi.waitFor(() => expect(register).toHaveBeenCalledTimes(1));
    await vi.waitFor(() => expect(establish).toHaveBeenCalledWith(user, 3600));
  });

  it("creates the first administrator without collecting an invite", async () => {
    vi.spyOn(session, "establish").mockImplementation(() => {});
    const register = vi.fn(() => ({ data: { register: { success: true, error: null, user: { ...user, role: "admin" }, tokens } } }));
    await renderWithProviders(<RegisterForm isSetup />, {
      routes: ["/login"],
      mocks: [{
        request: { query: RegisterDocument, variables: { input: { name: "Reader", email: "reader@example.com", password: "aaaaaaaaaa", inviteToken: null } } },
        result: register,
      }],
    });
    expect(screen.queryByLabelText("Invite code")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Set up Librarian" })).toBeInTheDocument();
    await fillAccount();
    await userEvent.click(screen.getByRole("button", { name: "Create administrator" }));
    await vi.waitFor(() => expect(register).toHaveBeenCalledTimes(1));
  });
});
