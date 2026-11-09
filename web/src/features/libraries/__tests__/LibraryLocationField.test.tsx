import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { LibraryLocationField } from "../LibraryLocationField";

const mocks = vi.hoisted(() => {
  const query = vi.fn();
  return { connect: vi.fn(), query, client: { query } };
});
vi.mock("@apollo/client/react", () => ({
  useApolloClient: () => mocks.client,
  useMutation: () => [mocks.connect],
}));
vi.mock("@/features/files/FolderPickerDialog", () => ({ FolderPickerDialog: () => null }));

Element.prototype.getAnimations ??= () => [];
const wrapper = ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider>;

function Harness({ initial }: { initial: string }) {
  const form = useAppForm({ ...formBehaviour, defaultValues: { path: initial } });
  return (
    <form.AppForm>
      <form.AppField name="path">{() => <LibraryLocationField required />}</form.AppField>
      <form.Subscribe selector={(state) => state.values.path}>{(path) => <output data-testid="path">{path}</output>}</form.Subscribe>
    </form.AppForm>
  );
}

const path = () => screen.getByTestId("path").textContent;

describe("choosing where a library's files are", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.query.mockResolvedValue({ data: { shareSignIn: { share: "smb://nas/media", saved: false, username: null } } });
  });

  it("is a folder on this server by default", async () => {
    render(<Harness initial="/data/media/Movies" />, { wrapper });
    expect(screen.getByRole("button", { name: "This server" })).toHaveAttribute("aria-pressed", "true");
    expect(await screen.findByDisplayValue("/data/media/Movies")).toBeInTheDocument();
    expect(screen.queryByText("Test connection")).not.toBeInTheDocument();
    expect(path()).toBe("/data/media/Movies");
  });

  it("builds the share address from the server, the share and the folder", async () => {
    render(<Harness initial="" />, { wrapper });
    await userEvent.click(screen.getByRole("button", { name: "Network share" }));
    await userEvent.type(screen.getByLabelText(/Server/), "10.0.1.190");
    expect(path()).toBe("");
    await userEvent.type(screen.getByLabelText(/^Share/), "media");
    expect(path()).toBe("smb://10.0.1.190/media");
    await userEvent.type(screen.getByLabelText(/Folder in the share/), "Movies/Kids");
    expect(path()).toBe("smb://10.0.1.190/media/Movies/Kids");
  });

  it("signs in to test the share, and says what it found", async () => {
    mocks.connect.mockResolvedValue({ data: { connectShare: { success: true, error: null, share: "smb://10.0.1.190/media", folders: 5 } } });
    render(<Harness initial="" />, { wrapper });
    await userEvent.click(screen.getByRole("button", { name: "Network share" }));
    await userEvent.type(screen.getByLabelText(/Server/), "10.0.1.190:1445");
    await userEvent.type(screen.getByLabelText(/^Share/), "media");
    await userEvent.type(screen.getByLabelText(/User name/), "toby");
    await userEvent.type(screen.getByLabelText(/^Password/), "hunter2");
    expect(screen.getByRole("button", { name: "Browse the share" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Test connection" }));
    await waitFor(() => expect(mocks.connect).toHaveBeenCalledWith({ variables: { input: { host: "10.0.1.190", port: 1445, share: "media", username: "toby", password: "hunter2", save: true } } }));
    expect(await screen.findByText(/Connected\. 5 folders at the top of the share\. The sign-in is saved\./)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Browse the share" })).toBeEnabled();
  });

  it("shows why a connection failed, in the server's words", async () => {
    mocks.connect.mockResolvedValue({ data: { connectShare: { success: false, error: "The server name 'truenas.localdomain' does not resolve from the machine Librarian runs on. Try its IP address instead", share: null, folders: 0 } } });
    render(<Harness initial="" />, { wrapper });
    await userEvent.click(screen.getByRole("button", { name: "Network share" }));
    await userEvent.type(screen.getByLabelText(/Server/), "truenas.localdomain");
    await userEvent.type(screen.getByLabelText(/^Share/), "media");
    await userEvent.click(screen.getByRole("button", { name: "Test connection" }));
    expect(await screen.findByText(/does not resolve from the machine Librarian runs on\. Try its IP address/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Browse the share" })).toBeDisabled();
  });

  it("opens an existing share library with its parts filled in and its saved sign-in found", async () => {
    mocks.query.mockResolvedValue({ data: { shareSignIn: { share: "smb://nas/media", saved: true, username: "toby" } } });
    render(<Harness initial="smb://nas/media/Movies" />, { wrapper });
    expect(screen.getByRole("button", { name: "Network share" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByDisplayValue("nas")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Movies")).toBeInTheDocument();
    expect(await screen.findByText(/A sign-in for toby is saved for this share/)).toBeInTheDocument();
    expect(screen.getByDisplayValue("toby")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "Browse the share" })).toBeEnabled());
    expect(path()).toBe("smb://nas/media/Movies");
  });

  it("clears the address when the kind of location changes", async () => {
    render(<Harness initial="/data/media/Movies" />, { wrapper });
    await userEvent.click(screen.getByRole("button", { name: "Network share" }));
    expect(path()).toBe("");
    await userEvent.click(screen.getByRole("button", { name: "This server" }));
    expect(path()).toBe("");
  });
});
