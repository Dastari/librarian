import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { LibraryLocationField } from "../LibraryLocationField";

const server = (id: string, name: string, host: string, extra: Record<string, unknown> = {}) => ({
  __typename: "NetworkServer", id, name, host, port: null, username: "toby", hasPassword: true, knownShares: ["media"],
  status: "ok", lastError: null, lastCheckedAt: null, createdAt: "", updatedAt: "", address: `smb://${host}`, ...extra,
});

const mocks = vi.hoisted(() => ({ servers: [] as unknown[], shares: [] as unknown[], update: vi.fn() }));
vi.mock("@apollo/client/react", () => ({
  useApolloClient: () => ({ query: vi.fn() }),
  useMutation: () => [mocks.update, { loading: false }],
}));
vi.mock("@/lib/graphql/live/hooks", () => ({
  useLiveQuery: (_document: unknown, options: { variables?: { id?: string }; skip?: boolean }) => {
    if (options.variables && "id" in options.variables) {
      return options.skip ? { data: undefined, loading: false } : { data: { networkServerShares: { success: true, error: null, shares: mocks.shares } }, loading: false, refetch: vi.fn() };
    }
    return { data: { networkServers: { edges: mocks.servers.map((node) => ({ node })) } }, loading: false, refetch: vi.fn() };
  },
}));
vi.mock("@/features/files/FolderPickerDialog", () => ({ FolderPickerDialog: () => null }));
vi.mock("@/features/servers/NetworkServerDialogs", () => ({ AddNetworkServerDialog: ({ open }: { open: boolean }) => (open ? <div role="dialog">Add network share</div> : null) }));

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
    mocks.servers = [server("s1", "TrueNAS", "10.0.1.190")];
    mocks.shares = [{ __typename: "NetworkShare", name: "media", comment: null, listed: true }, { __typename: "NetworkShare", name: "backups", comment: null, listed: true }];
  });

  it("is a folder on this server by default", async () => {
    render(<Harness initial="/data/media/Movies" />, { wrapper });
    expect(screen.getByRole("button", { name: "This server" })).toHaveAttribute("aria-pressed", "true");
    expect(await screen.findByDisplayValue("/data/media/Movies")).toBeInTheDocument();
    expect(path()).toBe("/data/media/Movies");
  });

  it("opens a library on a share with its saved server and share chosen, and never asks for a password", async () => {
    render(<Harness initial="smb://10.0.1.190/media/Movies" />, { wrapper });
    expect(screen.getByRole("button", { name: "Network share" })).toHaveAttribute("aria-pressed", "true");
    await waitFor(() => expect(screen.getByRole("combobox", { name: "Server" })).toHaveTextContent("TrueNAS"));
    expect(screen.getByRole("combobox", { name: "Share" })).toHaveTextContent("media");
    expect(screen.getByDisplayValue("Movies")).toBeInTheDocument();
    expect(screen.getByText("Signs in as toby")).toBeInTheDocument();
    expect(screen.queryByLabelText(/Password/)).not.toBeInTheDocument();
    await userEvent.clear(screen.getByDisplayValue("Movies"));
    await userEvent.type(screen.getByLabelText(/Folder in the share/), "TV Shows");
    expect(path()).toBe("smb://10.0.1.190/media/TV Shows");
  });

  it("offers to save a server when none is saved", async () => {
    mocks.servers = [];
    render(<Harness initial="" />, { wrapper });
    await userEvent.click(screen.getByRole("button", { name: "Network share" }));
    expect(screen.getByText(/No file server is saved yet/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Add network share/ }));
    expect(screen.getByRole("dialog")).toHaveTextContent("Add network share");
  });

  it("keeps the port of a server on another port in the address", async () => {
    mocks.servers = [server("s2", "Test", "127.0.0.1", { port: 4455, address: "smb://127.0.0.1:4455", knownShares: ["dl"] })];
    mocks.shares = [{ __typename: "NetworkShare", name: "dl", comment: null, listed: true }];
    render(<Harness initial="smb://127.0.0.1:4455/dl" />, { wrapper });
    await waitFor(() => expect(screen.getByRole("combobox", { name: "Server" })).toHaveTextContent("Test"));
    await userEvent.type(screen.getByLabelText(/Folder in the share/), "Downloads");
    expect(path()).toBe("smb://127.0.0.1:4455/dl/Downloads");
  });

  it("clears the address when the kind of location changes", async () => {
    render(<Harness initial="/data/media/Movies" />, { wrapper });
    await userEvent.click(screen.getByRole("button", { name: "Network share" }));
    expect(path()).toBe("");
    await userEvent.click(screen.getByRole("button", { name: "This server" }));
    expect(path()).toBe("");
  });
});
