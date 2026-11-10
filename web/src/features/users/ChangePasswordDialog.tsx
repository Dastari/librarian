import { useMutation } from "@apollo/client/react";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { currentPasswordSchema, passwordSchema } from "glass-ux/password-field";
import { toast } from "glass-ux/toast";
import { z } from "zod";

import { Dialog } from "@/components/ui";
import { ChangePasswordDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { errorMessage } from "@/lib/graphql/errors";

import { MIN_PASSWORD } from "./AccountRecoveryDialogs";

const schema = z.object({
  currentPassword: currentPasswordSchema(),
  newPassword: passwordSchema({ minLength: MIN_PASSWORD, required: true }),
});

/**
 * Change your password with the current one. Every other device is signed out; this one gets a
 * new session straight away.
 */
export function ChangePasswordDialog({ open, onOpenChange, onChanged, userInputs }: { open: boolean; onOpenChange: (open: boolean) => void; onChanged?: () => void; userInputs?: string[] }) {
  const [change] = useMutation(ChangePasswordDocument);
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: { currentPassword: "", newPassword: "" },
    validators: { onDynamic: schema },
    onSubmit: async ({ value }) => {
      const payload = (await change({ variables: { input: value } })).data?.changePassword;
      if (!payload?.success || !payload.user || !payload.tokens) throw new Error(payload?.error ?? "The password could not be changed");
      session.establish(payload.user, payload.tokens.expiresIn);
      void session.reconcile();
      toast.success("Password changed. Your other devices are signed out.");
      onChanged?.();
      onOpenChange(false);
    },
  });
  return (
    <Dialog isOpen={open} onOpenChange={onOpenChange} title="Change password" size="sm">
      <Form form={form} onError={(error) => toast.danger(errorMessage(error, "The password could not be changed"))}>
        <form.AppField name="currentPassword">{(field) => <field.PasswordField label="Current password" purpose="current" required />}</form.AppField>
        <form.AppField name="newPassword">{(field) => <field.PasswordField label="New password" purpose="new" userInputs={userInputs} required />}</form.AppField>
        <FormActions>
          <form.CancelButton onCancel={() => onOpenChange(false)}>Cancel</form.CancelButton>
          <form.SubmitButton>Change password</form.SubmitButton>
        </FormActions>
      </Form>
    </Dialog>
  );
}
