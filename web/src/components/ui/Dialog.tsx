import { Dialog as GlassDialog, type DialogSize } from "glass-ux/dialog";
import { useLayoutEffect, useRef, type ReactNode } from "react";

interface DialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  size?: "xs" | "sm" | "md" | "lg" | "xl" | "wide" | "full" | "cover";
  className?: string;
  isDismissable?: boolean;
}

const SIZE: Record<NonNullable<DialogProps["size"]>, DialogSize> = {
  xs: "sm", sm: "sm", md: "md", lg: "lg", xl: "xl", wide: "full", full: "full", cover: "full",
};

/** Controlled overlays have no library Trigger, so remember the host opener explicitly. */
export function useDialogReturnFocus(open: boolean) {
  const opener = useRef<HTMLElement | null>(null);
  useLayoutEffect(() => {
    if (open && document.activeElement instanceof HTMLElement && document.activeElement !== document.body) {
      opener.current = document.activeElement;
    }
  }, [open]);
  return () => opener.current?.isConnected ? opener.current : undefined;
}

export function Dialog({ isOpen, onOpenChange, title, description, children, footer, size = "md", className, isDismissable = true }: DialogProps) {
  const finalFocus = useDialogReturnFocus(isOpen);
  return (
    <GlassDialog.Root open={isOpen} disablePointerDismissal={!isDismissable} onOpenChange={(open, details) => {
      if (!open && !isDismissable) details.cancel();
      else onOpenChange(open);
    }}>
      <GlassDialog.Popup size={SIZE[size]} className={className} finalFocus={finalFocus}>
        <GlassDialog.Header title={title} close={isDismissable} />
        <GlassDialog.Body>
          {description ? <GlassDialog.Description>{description}</GlassDialog.Description> : null}
          {children}
        </GlassDialog.Body>
        {footer ? <GlassDialog.Footer>{footer}</GlassDialog.Footer> : null}
      </GlassDialog.Popup>
    </GlassDialog.Root>
  );
}
