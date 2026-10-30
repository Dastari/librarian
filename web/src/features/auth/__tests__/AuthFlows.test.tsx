import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { LoginDocument, RegisterDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { renderWithProviders } from "@/test";

import { LoginForm } from "../LoginForm";
import { RegisterForm } from "../RegisterForm";

const user = { id: "u-1", email: "reader@example.com", username: "reader", role: "user", displayName: "Reader" };
const tokens = { expiresIn: 3600, tokenType: "Bearer" };

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
        result: { data: { login: { success: true, error: null, user, tokens } } },
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
        result: { data: { login: { success: false, error: "Incorrect credentials", user: null, tokens: null } } },
      }],
    });
    await userEvent.type(screen.getByLabelText("Username or email"), "reader");
    await userEvent.type(screen.getByLabelText("Password", { exact: true }), "wrong");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByText("Incorrect credentials")).toBeInTheDocument();
    expect(establish).not.toHaveBeenCalled();
    expect(screen.queryByRole("link", { name: "Create an account" })).not.toBeInTheDocument();
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
