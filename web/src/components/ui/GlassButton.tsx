import { Button, type ButtonProps } from "./Button";

interface GlassButtonProps extends Omit<ButtonProps, "size"> {
  size?: "sm" | "md" | "lg" | "xl";
  emphasis?: "glass" | "brand";
  /** Kept for callers during migration; material rendering is owned by Glass UX. */
  refract?: boolean;
}

/** Button over media artwork, using the library's imagery treatment. */
export function GlassButton({ className, size = "md", emphasis = "glass", refract: _refract, ...props }: GlassButtonProps) {
  return (
    <span className="over-imagery contents">
      <Button {...props} size={size === "xl" ? "lg" : size} variant={emphasis === "brand" ? "primary" : "secondary"} className={className} />
    </span>
  );
}
