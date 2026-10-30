import { Dialog } from "glass-ux/dialog";
import { Notice } from "glass-ux/notice";
import { useRef, useState, type ReactNode } from "react";

import { errorMessage } from "@/lib/graphql/errors";

import { Button } from "./Button";
import { useDialogReturnFocus } from "./Dialog";

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

/** Host-owned confirmation lifecycle composed from Glass slots; callers close on success. */
export function ConfirmDialog({ isOpen, onOpenChange, title, description, confirmLabel = "Confirm", cancelLabel = "Cancel", destructive, isPending, onConfirm, children }: ConfirmDialogProps) {
  const cancel = useRef<HTMLButtonElement>(null);
  const finalFocus = useDialogReturnFocus(isOpen);
  const [running, setRunning] = useState(false);
  const [failure, setFailure] = useState<string>();
  const pending = isPending || running;

  async function confirm() {
    if (pending) return;
    setRunning(true);
    setFailure(undefined);
    try {
      await onConfirm();
    } catch (error) {
      setFailure(errorMessage(error));
    } finally {
      setRunning(false);
    }
  }

  return (
    <Dialog.Root open={isOpen} disablePointerDismissal onOpenChange={(open, details) => {
      if (!open && pending) details.cancel();
      else onOpenChange(open);
    }}>
      <Dialog.Popup size="sm" kind="alert" initialFocus={cancel} finalFocus={finalFocus}>
        <Dialog.Header title={title} tone={destructive ? "danger" : "default"} close={false} />
        <Dialog.Body>
          {description ? <Dialog.Description>{description}</Dialog.Description> : null}
          {children}
          {failure ? <Notice tone="danger">{failure}</Notice> : null}
        </Dialog.Body>
        <Dialog.Footer>
          <Button ref={cancel} variant="ghost" onPress={() => onOpenChange(false)} isDisabled={pending}>{cancelLabel}</Button>
          <Button variant={destructive ? "danger" : "primary"} onPress={() => void confirm()} isPending={pending}>{confirmLabel}</Button>
        </Dialog.Footer>
      </Dialog.Popup>
    </Dialog.Root>
  );
}
