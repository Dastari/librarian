import { Children, cloneElement, Fragment, isValidElement, type ReactNode } from "react";
import { Button as GlassButton, type ButtonProps as GlassButtonProps } from "glass-ux/button";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "danger-soft";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends Omit<GlassButtonProps, "variant" | "size"> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  isIconOnly?: boolean;
  fullWidth?: boolean;
  isPending?: boolean;
  isDisabled?: boolean;
  /** Temporary host spelling; Glass UX owns native pointer and keyboard activation. */
  onPress?: () => void;
}

/** Flatten only transparent fragments, preserving keys across fragment boundaries. */
function childParts(children: ReactNode, parentKey = ""): ReactNode[] {
  return Children.toArray(children).flatMap((node) => {
    if (!isValidElement<{ children?: ReactNode }>(node)) return [node];
    const key = `${parentKey}/${node.key}`;
    return node.type === Fragment
      ? childParts(node.props.children, key)
      : [cloneElement(node, { key })];
  });
}

/**
 * Legacy children use an SVG or childless component beside a label as an adornment.
 * This is a positional convention, not component-name detection: Tabler display names
 * omit "Icon", and component names can change in production. Rich component content
 * should use the public icon/iconEnd slots rather than this temporary convention.
 */
function isLegacyAdornment(node: ReactNode): boolean {
  if (!isValidElement<{ children?: ReactNode }>(node)) return false;
  return node.type === "svg" || (typeof node.type !== "string" && !Object.hasOwn(node.props, "children"));
}

/** Temporary host API bridge to the packaged button. */
export function Button({ children, variant = "secondary", size = "md", isIconOnly, fullWidth, isPending, isDisabled, onPress, onClick, disabled, status, icon, iconEnd, block, ...props }: ButtonProps) {
  const parts = childParts(children);
  const isWhitespace = (node: ReactNode) => typeof node === "string" && !node.trim();
  while (isWhitespace(parts[0])) parts.shift();
  while (isWhitespace(parts.at(-1))) parts.pop();
  const leadingIcon = !isIconOnly && icon === undefined && parts.length > 1 && isLegacyAdornment(parts[0]) ? parts.shift() : undefined;
  const trailingIcon = !isIconOnly && iconEnd === undefined && parts.length > 1 && isLegacyAdornment(parts.at(-1)) ? parts.pop() : undefined;
  const label = parts.length ? parts : undefined;
  return (
    <GlassButton
      {...props}
      variant={variant}
      size={size}
      block={fullWidth ?? block}
      disabled={isDisabled || disabled || isPending}
      status={isPending ? "pending" : (status ?? (isPending === undefined ? undefined : "idle"))}
      icon={icon !== undefined ? icon : (isIconOnly ? label : leadingIcon)}
      iconEnd={iconEnd !== undefined ? iconEnd : trailingIcon}
      data-focusable
      onClick={(event) => {
        onClick?.(event);
        if (!event.defaultPrevented) onPress?.();
      }}
    >
      {isIconOnly ? undefined : label}
    </GlassButton>
  );
}
