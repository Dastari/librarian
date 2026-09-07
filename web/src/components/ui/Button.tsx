import { Button as AriaButton, type ButtonProps as AriaButtonProps } from "react-aria-components";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

import { Spinner } from "./Spinner";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "danger-soft";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends Omit<AriaButtonProps, "children" | "className" | "style"> {
  children?: ReactNode;
  className?: string;
  variant?: ButtonVariant;
  size?: ButtonSize;
  isIconOnly?: boolean;
  fullWidth?: boolean;
  /** Shows a spinner and blocks presses while an action is in flight. */
  isPending?: boolean;
}

const SIZE: Record<ButtonSize, string> = {
  sm: "h-8 min-w-8 px-3 text-label gap-1.5 [&_svg]:size-4",
  md: "h-10 min-w-10 px-4 text-body-sm font-medium gap-2 [&_svg]:size-[18px]",
  lg: "h-12 min-w-12 px-6 text-body font-semibold gap-2.5 [&_svg]:size-5",
};

const VARIANT: Record<ButtonVariant, string> = {
  primary: "glass-control glass-brand hover-capable:hover:brightness-105",
  secondary: "glass-control text-foreground hover-capable:hover:bg-glass-fill-strong",
  ghost: "text-foreground/85 hover-capable:hover:bg-glass-fill hover-capable:hover:text-foreground",
  danger: "glass-control bg-danger! border-danger! text-danger-foreground hover-capable:hover:brightness-105",
  "danger-soft": "glass-control glass-danger hover-capable:hover:brightness-105",
};

/**
 * The application button. Every variant is built on the liquid-glass control tier so buttons
 * feel like the switches, tabs and inputs around them.
 */
export function Button({ children, className, variant = "secondary", size = "md", isIconOnly, fullWidth, isPending, isDisabled, ...props }: ButtonProps) {
  return (
    <AriaButton
      {...props}
      isDisabled={isDisabled || isPending}
      data-focusable
      className={cn(
        "nav-focus relative inline-flex shrink-0 items-center justify-center rounded-pill whitespace-nowrap transition-[transform,filter,background-color] duration-fast ease-fluid",
        "pressed:scale-[0.97] disabled:pointer-events-none disabled:opacity-50",
        SIZE[size],
        VARIANT[variant],
        isIconOnly && "aspect-square px-0",
        fullWidth && "w-full",
        className,
      )}
    >
      {isPending ? <Spinner size={size === "lg" ? 20 : 16} className="absolute" /> : null}
      <span className={cn("inline-flex items-center justify-center gap-[inherit]", isPending && "invisible")}>{children}</span>
    </AriaButton>
  );
}
