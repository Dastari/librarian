import { useMutation } from "@apollo/client/react";
import { Link } from "@tanstack/react-router";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { passwordSchema } from "glass-ux/password-field";
import { useState } from "react";
import { z } from "zod";

import { ResetPasswordWithLinkDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

const schema = z.object({ password: passwordSchema({ minLength: 10, required: true }) });

/**
 * Choose a new password with a reset link. The link works once and for a day; using it signs out
 * every device, and two-factor sign-in still applies when you sign in.
 */
export function ResetPasswordForm({ token }: { token: string }) {
  const [reset] = useMutation(ResetPasswordWithLinkDocument);
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: { password: "" },
    validators: { onDynamic: schema },
    onSubmit: async ({ value }) => {
      setError(null);
      const result = (await reset({ variables: { input: { token, newPassword: value.password } } })).data?.resetPasswordWithLink;
      if (!result?.success) throw new Error(result?.error ?? "The password could not be changed");
      setDone(true);
    },
  });
  const signIn = <Link to="/login" className="nav-focus rounded text-brand hover:underline">Sign in</Link>;
  if (!token) {
    return <div className="flex flex-col gap-3"><h1 className="text-title text-foreground">Reset your password</h1><p className="text-body-sm text-muted">This page needs the link an administrator gave you. Ask them for a new one.</p>{signIn}</div>;
  }
  if (done) {
    return <div className="flex flex-col gap-3"><h1 className="text-title text-foreground">Password changed</h1><p className="text-body-sm text-muted">Every device was signed out. Sign in with your new password.</p>{signIn}</div>;
  }
  return (
    <div className="flex min-w-0 flex-col gap-4">
      <h1 className="text-title text-foreground">Choose a new password</h1>
      {error ? <p role="alert" className="text-body-sm text-danger">{error}</p> : null}
      <Form form={form} onError={(failure) => setError(errorMessage(failure, "The password could not be changed"))}>
        <form.AppField name="password">{(field) => <field.PasswordField label="New password" purpose="new" required />}</form.AppField>
        <FormActions>
          <form.SubmitButton>Change password</form.SubmitButton>
        </FormActions>
      </Form>
    </div>
  );
}
