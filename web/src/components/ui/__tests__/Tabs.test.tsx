import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IconMovie } from "@tabler/icons-react";
import { describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { SegmentTabs, SideTabs, type TabItem } from "../Tabs";

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
    expect(disabled).toBeDisabled();
    await userEvent.click(disabled);
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("follows the URL when the items are links, preferring the longest match", async () => {
    await renderWithProviders(<SegmentTabs ariaLabel="Settings" items={navigation} />, { path: "/settings/quality" });
    expect(screen.getByRole("tab", { name: /Quality/ })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: /General/ })).toHaveAttribute("aria-selected", "false");
    expect(screen.getByRole("tab", { name: /Quality/ })).toHaveAttribute("href", "/settings/quality");
  });
});

describe("SideTabs", () => {
  it("renders links and marks the current page", async () => {
    await renderWithProviders(<SideTabs ariaLabel="Settings sections" items={[...navigation, { key: "downloads", label: "Downloads", href: "/settings/downloads", count: 3 }]} />, { path: "/settings/sources" });
    expect(screen.getByRole("navigation", { name: "Settings sections" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /Sources/ })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: /Quality/ })).not.toHaveAttribute("aria-current");
    expect(screen.getByRole("link", { name: /Downloads/ })).toHaveTextContent("3");
  });
});
