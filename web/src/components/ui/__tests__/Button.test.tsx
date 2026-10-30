import { createRef, Fragment, memo, type ReactNode } from "react";
import { IconBookmark, IconBookmarkFilled, IconChevronDown, IconPlayerPlay } from "@tabler/icons-react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IconPlay, IconPlus } from "glass-ux/icons";
import { TooltipProvider } from "glass-ux/tooltip";
import { describe, expect, it, vi } from "vitest";

import { Button } from "../Button";

describe("Button", () => {
  it("renders its label and fires onPress", async () => {
    const onPress = vi.fn();
    render(<Button onPress={onPress}>Save</Button>);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onPress).toHaveBeenCalledOnce();
  });

  it("maps variants and sizes to the library contract", () => {
    const { rerender } = render(<Button variant="primary">Go</Button>);
    expect(screen.getByRole("button")).toHaveAttribute("data-variant", "primary");
    rerender(<Button variant="danger" size="lg">Go</Button>);
    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("data-variant", "danger");
    expect(button).toHaveAttribute("data-size", "lg");
  });

  it("is focusable by the spatial navigator", () => {
    render(<Button>Go</Button>);
    expect(screen.getByRole("button")).toHaveAttribute("data-focusable");
  });

  it("blocks presses while pending and keeps focus available", async () => {
    const onPress = vi.fn();
    render(<Button isPending onPress={onPress}>Save</Button>);
    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(button).toHaveAttribute("aria-busy", "true");
    await userEvent.tab();
    expect(button).toHaveFocus();
    await userEvent.click(button);
    expect(onPress).not.toHaveBeenCalled();
    expect(button).toHaveTextContent("Save");
  });

  it("supports native keyboard activation once per press", async () => {
    const onPress = vi.fn();
    render(<Button onPress={onPress}>Save</Button>);
    screen.getByRole("button", { name: "Save" }).focus();
    await userEvent.keyboard("{Enter} ");
    expect(onPress).toHaveBeenCalledTimes(2);
  });

  it("blocks presses while disabled", async () => {
    const onPress = vi.fn();
    render(<Button isDisabled onPress={onPress}>Save</Button>);
    await userEvent.click(screen.getByRole("button"));
    expect(onPress).not.toHaveBeenCalled();
  });

  it.each([
    ["Tabler", <IconPlayerPlay key="play" />],
    ["Glass", <IconPlay key="play" />],
    ["SVG", <svg key="play" data-testid="raw-icon"><path d="M7 4v16l13-8z" /></svg>],
  ])("puts a leading %s icon in the public icon slot, outside the label", (_name, icon) => {
    render(<Button>{icon} Play</Button>);
    const button = screen.getByRole("button", { name: "Play" });
    expect(button.querySelector("svg")?.parentElement).toBe(button);
    expect(button.querySelector(".fa-btn-label svg")).toBeNull();
    expect(button.querySelector(".fa-btn-label")).toHaveTextContent("Play");
  });

  it("extracts fragment-wrapped leading and trailing icons without moving rich labels", () => {
    render(
      <Button>
        {" "}
        <Fragment key="lead"><><IconPlayerPlay /></></Fragment>
        <Fragment key="label"><span>Play</span></Fragment>
        <Fragment key="end"><><IconChevronDown /></></Fragment>
        {" "}
      </Button>,
    );
    const button = screen.getByRole("button", { name: "Play" });
    expect(button.firstElementChild).toHaveClass("tabler-icon-player-play");
    expect(button.lastElementChild).toHaveClass("tabler-icon-chevron-down");
    expect(button.querySelector(".fa-btn-label span")).toHaveTextContent("Play");
    expect(button.querySelector(".fa-btn-label svg")).toBeNull();
  });

  it("normalizes conditional icons and labels on each render", () => {
    const content = (wanted: boolean) => (
      <Button isPending={false}>
        {false}{null}{undefined}
        <>{wanted ? <IconBookmarkFilled /> : <IconBookmark />} {wanted ? "Wanted" : "Want"}</>
      </Button>
    );
    const { rerender } = render(content(false));
    const button = screen.getByRole("button", { name: "Want" });
    expect(button.querySelector(".fa-btn-face[data-active] > svg")).toHaveClass("tabler-icon-bookmark");
    expect(button.querySelector(".fa-btn-label svg")).toBeNull();
    rerender(content(true));
    expect(screen.getByRole("button", { name: "Wanted" })).toBe(button);
    expect(button.querySelector(".fa-btn-face[data-active] > svg")).toHaveClass("tabler-icon-bookmark-filled");
    expect(button.querySelector(".fa-btn-label svg")).toBeNull();
  });

  it("keeps the label when a conditional icon disappears", () => {
    const content = (showIcon: boolean) => <Button><>{showIcon && <IconPlay />} Play {showIcon && <IconChevronDown />}</></Button>;
    const { rerender } = render(content(true));
    expect(screen.getByRole("button").querySelectorAll("svg")).toHaveLength(2);
    rerender(content(false));
    expect(screen.getByRole("button", { name: "Play" }).querySelector("svg")).toBeNull();
  });

  it("supports memoized icons without inspecting component names", () => {
    const Play = memo(IconPlayerPlay);
    render(<Button><Play /> Play</Button>);
    const button = screen.getByRole("button", { name: "Play" });
    expect(button.firstElementChild).toHaveClass("tabler-icon-player-play");
    expect(button.querySelector(".fa-btn-label svg")).toBeNull();
  });

  it("keeps explicit public icon slots and rich labels intact", () => {
    function Label() { return <strong>Play</strong>; }
    render(<Button icon={<IconPlay />} iconEnd={<IconChevronDown />}><Label /></Button>);
    const button = screen.getByRole("button", { name: "Play" });
    expect(button.firstElementChild).toHaveAttribute("data-icon", "play");
    expect(button.lastElementChild).toHaveClass("tabler-icon-chevron-down");
    expect(button.querySelector(".fa-btn-label strong")).toHaveTextContent("Play");
    expect(button.querySelectorAll("svg")).toHaveLength(2);
    expect(button.querySelector(".fa-btn-label svg")).toBeNull();
  });

  it("does not mistake named rich content for a legacy icon", () => {
    function IconLabel({ children }: { children: ReactNode }) { return <strong>{children}</strong>; }
    render(<Button><IconLabel>Play</IconLabel> now</Button>);
    const button = screen.getByRole("button", { name: "Play now" });
    expect(button.querySelector(".fa-btn-label strong")).toHaveTextContent("Play");
  });

  it.each(["legacy", "public"])("keeps %s icon-only buttons label-free through status changes", async (api) => {
    const onPress = vi.fn();
    const ref = createRef<HTMLElement>();
    const content = (isPending: boolean) => (
      <TooltipProvider>
        <Button ref={ref} isIconOnly={api === "legacy"} icon={api === "public" ? <IconPlus /> : undefined} isPending={isPending} aria-label="Add" onPress={onPress}>
          {api === "legacy" ? <><IconPlus /></> : <>{false}{null}</>}
        </Button>
      </TooltipProvider>
    );
    const { rerender } = render(content(false));
    const button = screen.getByRole("button", { name: "Add" });
    expect(ref.current).toBe(button);
    expect(button).toHaveAttribute("data-icon-only");
    expect(button.querySelector(".fa-btn-label")).toBeNull();
    await userEvent.tab();
    expect(button).toHaveFocus();
    rerender(content(true));
    expect(ref.current).toBe(button);
    expect(button).toHaveFocus();
    expect(button).toHaveAttribute("aria-busy", "true");
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(button.querySelector(".fa-btn-label")).toBeNull();
    await userEvent.keyboard("{Enter} ");
    expect(onPress).not.toHaveBeenCalled();
    rerender(content(false));
    expect(button).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(onPress).toHaveBeenCalledOnce();
  });

  it("preserves public status feedback alongside the pending alias", async () => {
    const content = (status: "pending" | "success") => <Button isPending={false} status={status} successLabel="Saved" icon={<IconPlus />}>Save</Button>;
    const { rerender } = render(content("pending"));
    const button = screen.getByRole("button", { name: "Save" });
    await userEvent.tab();
    expect(button).toHaveFocus();
    expect(button).toHaveAttribute("aria-busy", "true");
    rerender(content("success"));
    expect(screen.getByRole("button", { name: "Saved" })).toBe(button);
    expect(button).toHaveFocus();
    expect(button).toHaveAttribute("data-status", "success");
    expect(button).not.toHaveAttribute("aria-disabled");
    expect(button.querySelector(".fa-btn-label svg")).toBeNull();
  });

  it("lets onClick cancel the legacy onPress callback", async () => {
    const onPress = vi.fn();
    render(<Button onClick={(event) => event.preventDefault()} onPress={onPress}>Save</Button>);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onPress).not.toHaveBeenCalled();
  });
});
