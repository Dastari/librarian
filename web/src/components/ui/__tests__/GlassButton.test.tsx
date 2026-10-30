import { createRef } from "react";
import { IconPlayerPlay } from "@tabler/icons-react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IconPlay } from "glass-ux/icons";
import { describe, expect, it, vi } from "vitest";

import { GlassButton } from "../GlassButton";

describe("GlassButton", () => {
  it("puts the control inside the public imagery treatment and preserves caller props", async () => {
    const onPress = vi.fn();
    const ref = createRef<HTMLElement>();
    render(<GlassButton ref={ref} className="shrink-0" emphasis="brand" size="xl" icon={<IconPlay />} onPress={onPress}>Play</GlassButton>);
    const button = screen.getByRole("button", { name: "Play" });
    expect(button.parentElement).toHaveClass("over-imagery", "contents");
    expect(button).not.toHaveClass("over-imagery");
    expect(button).toHaveClass("shrink-0");
    expect(button).toHaveAttribute("data-variant", "primary");
    expect(button).toHaveAttribute("data-size", "lg");
    expect(ref.current).toBe(button);
    expect(button.querySelector(".fa-btn-label svg")).toBeNull();
    await userEvent.tab();
    expect(button).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(onPress).toHaveBeenCalledOnce();
  });

  it("keeps legacy Tabler icons outside the media button label", () => {
    render(<GlassButton><><IconPlayerPlay /> Play</></GlassButton>);
    const button = screen.getByRole("button", { name: "Play" });
    expect(button).toHaveAttribute("data-variant", "secondary");
    expect(button.firstElementChild).toHaveClass("tabler-icon-player-play");
    expect(button.querySelector(".fa-btn-label svg")).toBeNull();
  });
});
