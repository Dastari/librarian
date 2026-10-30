import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { BrowseDirectoryDocument, type BrowseDirectoryQuery } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { FolderPickerDialog } from "../FolderPickerDialog";

type Directory = BrowseDirectoryQuery["browseDirectory"];
const entry = (name: string, path: string, readable = true): Directory["entries"][number] => ({ name, path, readable, writable: readable, isDir: true, size: 0, sizeFormatted: "0 B", mimeType: null, modifiedAt: null });
const listing = (currentPath: string, entries: Directory["entries"] = []): Directory => ({ currentPath, entries, parentPath: "/", isLibraryPath: false, libraryId: null, quickPaths: [] });
const request = (path: string) => ({ query: BrowseDirectoryDocument, variables: { input: { path, dirsOnly: true, showHidden: false } } });

async function mount(mocks: Parameters<typeof renderWithProviders>[1], initialPath = "/media") {
  const pick = vi.fn(), close = vi.fn();
  await renderWithProviders(<FolderPickerDialog isOpen initialPath={initialPath} onPick={pick} onOpenChange={close} />, mocks);
  return { pick, close };
}

describe("FolderPickerDialog Glass rows", () => {
  it("uses the server's normalized current path rather than the requested spelling", async () => {
    const view = await mount({ mocks: [{ request: request("/media/."), result: { data: { browseDirectory: listing("/media") } } }] }, "/media/.");
    const use = await screen.findByRole("button", { name: "Use this folder" });
    await vi.waitFor(() => expect(use).toBeEnabled());
    use.focus();
    await userEvent.keyboard("{Enter}");
    expect(view.pick).toHaveBeenCalledWith("/media");
    expect(view.close).toHaveBeenCalledWith(false);
  });

  it("keeps a readable row available for double click while the next directory loads", async () => {
    const view = await mount({ mocks: [
      { request: request("/media"), result: { data: { browseDirectory: listing("/media", [entry("Films", "/media/films")]) } } },
      { request: request("/media/films"), delay: 500, result: { data: { browseDirectory: listing("/media/films") } } },
    ] });
    await userEvent.dblClick(await screen.findByRole("button", { name: "Films" }));
    expect(view.pick).toHaveBeenCalledTimes(1);
    expect(view.pick).toHaveBeenCalledWith("/media/films");
    expect(view.close).toHaveBeenCalledWith(false);
  });

  it("does not navigate or select an unreadable row", async () => {
    const view = await mount({ mocks: [{ request: request("/media"), result: { data: { browseDirectory: listing("/media", [entry("Private", "/media/private", false)]) } } }] });
    const privateFolder = await screen.findByRole("button", { name: "Private" });
    expect(privateFolder).toBeDisabled();
    await userEvent.dblClick(privateFolder);
    expect(view.pick).not.toHaveBeenCalled();
    expect(view.close).not.toHaveBeenCalled();
  });

  it("disables selection of the previous path after browsing fails", async () => {
    const view = await mount({ mocks: [
      { request: request("/media"), result: { data: { browseDirectory: listing("/media", [entry("Broken folder", "/broken")]) } } },
      { request: request("/broken"), error: new Error("Permission denied") },
    ] });
    await userEvent.click(await screen.findByRole("button", { name: "Broken folder" }));
    expect(await screen.findByText("Can't read this folder")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Use this folder" })).toBeDisabled();
    expect(view.pick).not.toHaveBeenCalled();
  });
});
