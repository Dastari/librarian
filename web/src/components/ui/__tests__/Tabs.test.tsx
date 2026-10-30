import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IconMovie } from "@tabler/icons-react";
import { describe, expect, it, vi } from "vitest";

import { useSpatialNavigation } from "@/lib/input-mode";
import { renderWithProviders } from "@/test";

import { SegmentTabs, SideTabs, type TabItem } from "../Tabs";
import { ViewToggle } from "../Toolbar";

const controlled: TabItem[] = [
  { key: "all", label: "All" },
  { key: "wanted", label: "Wanted", count: 12 },
  { key: "ignored", label: "Ignored", disabled: true },
];

const navigation: TabItem[] = [
  { key: "general", label: "General", href: "/settings", icon: IconMovie },
  { key: "quality", label: "Quality", href: "/settings/quality" },
  { key: "sources", label: "Sources", href: "/settings/sources" },
];

describe("SegmentTabs", () => {
  it("is a tab list with one selected tab", async () => {
    await renderWithProviders(<SegmentTabs ariaLabel="Views" items={controlled} selected="wanted" onSelect={() => {}} />);
    expect(screen.getByRole("tablist", { name: "Views" })).toBeInTheDocument();
    expect(screen.getAllByRole("tab")).toHaveLength(3);
    expect(screen.getByRole("tab", { name: /Wanted/ })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: "All" })).toHaveAttribute("aria-selected", "false");
  });

  it("shows a count badge when one is supplied", async () => {
    await renderWithProviders(<SegmentTabs ariaLabel="Views" items={controlled} selected="all" onSelect={() => {}} />);
    expect(screen.getByRole("tab", { name: /Wanted/ })).toHaveTextContent("12");
  });

  it("reports the picked tab and skips disabled ones with the arrow keys", async () => {
    const onSelect = vi.fn();
    await renderWithProviders(<SegmentTabs ariaLabel="Views" items={controlled} selected="all" onSelect={onSelect} />);
    await userEvent.click(screen.getByRole("tab", { name: /Wanted/ }));
    expect(onSelect).toHaveBeenCalledWith("wanted");

    screen.getByRole("tab", { name: "All" }).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(onSelect).toHaveBeenLastCalledWith("wanted");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: /Wanted/ }));
  });

  it("does not select a disabled tab", async () => {
    const onSelect = vi.fn();
    await renderWithProviders(<SegmentTabs ariaLabel="Views" items={controlled} selected="all" onSelect={onSelect} />);
    const disabled = screen.getByRole("tab", { name: "Ignored" });
    expect(disabled).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(disabled);
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("follows the URL when the items are links, preferring the longest match", async () => {
    await renderWithProviders(<SegmentTabs ariaLabel="Settings" items={navigation} />, { path: "/settings/quality" });
    expect(screen.getByRole("link", { name: /Quality/ })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: /General/ })).not.toHaveAttribute("aria-current");
    expect(screen.getByRole("link", { name: /Quality/ })).toHaveAttribute("href", "/settings/quality");
  });
});

describe("SideTabs", () => {
  it("keeps spatial navigation available in its page content", async () => {
    function Content() {
      useSpatialNavigation();
      return <><button data-testid="save">Save</button><button data-testid="next">Next action</button></>;
    }
    await renderWithProviders(<SideTabs ariaLabel="Settings sections" items={navigation}><Content /></SideTabs>, { path: "/settings" });
    const save = screen.getByTestId("save"), next = screen.getByTestId("next");
    const box = (x: number) => ({ x, y: 0, left: x, right: x + 40, top: 0, bottom: 20, width: 40, height: 20, toJSON: () => ({}) });
    save.getBoundingClientRect = () => box(0);
    next.getBoundingClientRect = () => box(100);
    save.focus();
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(next).toHaveFocus();
  });

  it("renders links and marks the current page", async () => {
    await renderWithProviders(<SideTabs ariaLabel="Settings sections" items={[...navigation, { key: "downloads", label: "Downloads", href: "/settings/downloads", count: 3 }]} />, { path: "/settings/sources" });
    expect(screen.getByRole("navigation", { name: "Settings sections" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /Sources/ })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: /Quality/ })).not.toHaveAttribute("aria-current");
    expect(screen.getByRole("link", { name: /Downloads/ })).toHaveTextContent("3");
  });
});

describe("ViewToggle", () => {
  it("keeps accessible view labels and reports the host view mode", async () => {
    const onChange = vi.fn();
    await renderWithProviders(<ViewToggle value="card" onChange={onChange} />);
    expect(screen.getByRole("button", { name: "Grid view" })).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(screen.getByRole("button", { name: "List view" }));
    expect(onChange).toHaveBeenCalledWith("table");
  });
});
