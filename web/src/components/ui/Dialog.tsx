import { Modal } from "@heroui/react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface DialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  /** `xl` and `wide` are Librarian additions for dialogs that contain tables. */
  size?: "xs" | "sm" | "md" | "lg" | "xl" | "wide" | "full" | "cover";
  className?: string;
  isDismissable?: boolean;
}

/** Standard glass dialog. Forms and pickers use this; confirmations use ConfirmDialog. */
export function Dialog({ isOpen, onOpenChange, title, description, children, footer, size = "md", className, isDismissable = true }: DialogProps) {
  return (
    <Modal isOpen={isOpen} onOpenChange={onOpenChange}>
      <Modal.Backdrop variant="blur" isDismissable={isDismissable}>
        <Modal.Container size={size === "xl" || size === "wide" ? "lg" : size} placement="auto" scroll="inside" className={cn(size === "xl" && "[&_.modal__dialog]:!max-w-[min(64rem,94vw)]", size === "wide" && "[&_.modal__dialog]:!max-w-[min(84rem,96vw)]")}>
          <Modal.Dialog className={cn("glass-surface glass-highlight overflow-hidden", size === "xl" && "!max-w-[min(64rem,94vw)]", size === "wide" && "!max-w-[min(84rem,96vw)]", className)}>
            <Modal.Header>
              <Modal.Heading>{title}</Modal.Heading>
              {description ? <p className="mt-1 text-body-sm text-muted">{description}</p> : null}
              <Modal.CloseTrigger />
            </Modal.Header>
            <Modal.Body className="scrollbar-thin overflow-x-hidden">{children}</Modal.Body>
            {footer ? <Modal.Footer>{footer}</Modal.Footer> : null}
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </Modal>
  );
}
