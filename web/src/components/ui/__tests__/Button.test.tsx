import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Button } from "../Button";

describe("Button", () => {
  it("renders its label and fires onPress", async () => {
    const onPress = vi.fn();
    render(<Button onPress={onPress}>Save</Button>);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onPress).toHaveBeenCalledOnce();
  });

  it("applies the variant and size classes", () => {
    const { rerender } = render(<Button variant="primary">Go</Button>);
    expect(screen.getByRole("button").className).toContain("glass-brand");
    rerender(<Button variant="danger" size="lg">Go</Button>);
    const button = screen.getByRole("button");
    expect(button.className).toContain("bg-danger!");
    expect(button.className).toContain("h-12");
  });

  it("is focusable by the spatial navigator", () => {
    render(<Button>Go</Button>);
    expect(screen.getByRole("button")).toHaveAttribute("data-focusable");
  });

  it("blocks presses while pending and hides the label behind a spinner", async () => {
    const onPress = vi.fn();
    render(<Button isPending onPress={onPress}>Save</Button>);
    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
    await userEvent.click(button);
    expect(onPress).not.toHaveBeenCalled();
    expect(screen.getByText("Save").className).toContain("invisible");
  });

  it("blocks presses while disabled", async () => {
    const onPress = vi.fn();
    render(<Button isDisabled onPress={onPress}>Save</Button>);
    await userEvent.click(screen.getByRole("button"));
    expect(onPress).not.toHaveBeenCalled();
  });
});
