import { Modal } from "@heroui/react";

import { Button } from "./Button";
import type { ReactNode } from "react";

interface ConfirmDialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
  isPending?: boolean;
  onConfirm: () => void | Promise<void>;
  children?: ReactNode;
}

/** Confirmation for destructive or irreversible actions. Keeps the wording to one sentence. */
export function ConfirmDialog({
  isOpen,
  onOpenChange,
  title,
  description,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  destructive,
  isPending,
  onConfirm,
  children,
}: ConfirmDialogProps) {
  return (
    <Modal isOpen={isOpen} onOpenChange={onOpenChange}>
      <Modal.Backdrop variant="blur">
        <Modal.Container size="sm" placement="center">
          <Modal.Dialog className="glass-surface glass-highlight overflow-hidden">
            <Modal.Header>
              <Modal.Heading>{title}</Modal.Heading>
            </Modal.Header>
            <Modal.Body>
              {description ? <p className="text-body-sm text-muted">{description}</p> : null}
              {children}
            </Modal.Body>
            <Modal.Footer>
              <Button variant="ghost" onPress={() => onOpenChange(false)} isDisabled={isPending}>
                {cancelLabel}
              </Button>
              <Button variant={destructive ? "danger" : "primary"} onPress={() => void onConfirm()} isPending={isPending}>
                {confirmLabel}
              </Button>
            </Modal.Footer>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </Modal>
  );
}
