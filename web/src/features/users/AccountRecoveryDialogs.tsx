import { useMutation } from "@apollo/client/react";
import { CopyButton } from "glass-ux/copy-button";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { passwordSchema } from "glass-ux/password-field";
import { Surface } from "glass-ux/surface";
import { toast } from "glass-ux/toast";
import { useState } from "react";
import { z } from "zod";

import { Button, Dialog } from "@/components/ui";
import { CreatePasswordResetLinkDocument, SetTemporaryPasswordDocument } from "@/graphql/generated/graphql";
import { formatDateTime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { userName, type UserEntity } from "./UserIdentity";

/** Registration's floor, so a temporary password is never weaker than a chosen one. */
export const MIN_PASSWORD = 10;

const temporarySchema = z.object({ password: passwordSchema({ minLength: MIN_PASSWORD, minScore: 1, required: true }) });

/**
 * An administrator gives someone a temporary password to pass on. Their sessions end, and their
 * next sign-in asks them to choose their own before it signs them in (design.md Q148).
 */
export function TemporaryPasswordDialog({ user, open, onOpenChange, onChanged }: { user: UserEntity; open: boolean; onOpenChange: (open: boolean) => void; onChanged?: () => void }) {
  const [set] = useMutation(SetTemporaryPasswordDocument);
  const [given, setGiven] = useState<string | null>(null);
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: { password: "" },
    validators: { onDynamic: temporarySchema },
    onSubmit: async ({ value }) => {
      assertSuccess((await set({ variables: { input: { userId: user.id, password: value.password } } })).data?.setTemporaryPassword, "The temporary password could not be set");
      setGiven(value.password);
      onChanged?.();
    },
  });
  const name = userName(user);
  return (
    <Dialog isOpen={open} onOpenChange={onOpenChange} title={`Temporary password for ${name}`} size="sm"
      footer={given ? <Button variant="primary" onPress={() => onOpenChange(false)}>Done</Button> : undefined}>
      {given ? (
        <div className="flex min-w-0 flex-col gap-3">
          <p className="text-body-sm text-muted">{name} is signed out everywhere. Give them this password; they choose a new one when they next sign in.</p>
          <Surface material="inset" padding="tile" radius="tile" className="flex min-w-0 items-center gap-2">
            <code className="min-w-0 flex-1 font-mono text-body [overflow-wrap:anywhere]">{given}</code>
            <CopyButton value={given} label="Copy password" />
          </Surface>
        </div>
      ) : (
        <Form form={form} onError={(error) => toast.danger(errorMessage(error, "The temporary password could not be set"))}>
          <p className="text-body-sm text-muted">{name} is signed out everywhere and must choose a new password at their next sign-in.</p>
          <form.AppField name="password">{(field) => <field.PasswordField label="Temporary password" purpose="new" generate required />}</form.AppField>
          <FormActions>
            <form.CancelButton onCancel={() => onOpenChange(false)}>Cancel</form.CancelButton>
            <form.SubmitButton>Set password</form.SubmitButton>
          </FormActions>
        </Form>
      )}
    </Dialog>
  );
}

/**
 * An administrator makes a one-time reset link for someone. Nothing sends mail, so the link is
 * shown here to pass on. It works once, for a day, and replaces any earlier link.
 */
export function ResetLinkDialog({ user, open, onOpenChange }: { user: UserEntity; open: boolean; onOpenChange: (open: boolean) => void }) {
  const [create, { loading }] = useMutation(CreatePasswordResetLinkDocument);
  const [link, setLink] = useState<{ url: string; expiresAt: string | null } | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const name = userName(user);
  const make = async () => {
    setProblem(null);
    try {
      const result = assertSuccess((await create({ variables: { userId: user.id } })).data?.createPasswordResetLink, "The link could not be made");
      if (!result.path) throw new Error("The link could not be made");
      setLink({ url: `${window.location.origin}${result.path}`, expiresAt: result.expiresAt ?? null });
    } catch (error) {
      setProblem(errorMessage(error));
    }
  };
  return (
    <Dialog isOpen={open} onOpenChange={onOpenChange} title={`Reset link for ${name}`} size="sm"
      footer={link ? <Button variant="primary" onPress={() => onOpenChange(false)}>Done</Button> : <><Button variant="ghost" onPress={() => onOpenChange(false)}>Cancel</Button><Button variant="primary" isPending={loading} onPress={() => void make()}>Make link</Button></>}>
      <div className="flex min-w-0 flex-col gap-3">
        {link ? (
          <>
            <p className="text-body-sm text-muted">Send this to {name} yourself. It works once{link.expiresAt ? `, until ${formatDateTime(link.expiresAt)}` : ""}. Opening it lets them choose a new password; their sessions end when they do.</p>
            <Surface material="inset" padding="tile" radius="tile" className="flex min-w-0 items-center gap-2">
              <code className="min-w-0 flex-1 font-mono text-body-sm [overflow-wrap:anywhere]">{link.url}</code>
              <CopyButton value={link.url} label="Copy link" />
            </Surface>
          </>
        ) : (
          <p className="text-body-sm text-muted">A link that lets {name} choose a new password. It works once, for a day, and replaces any link made before. Librarian doesn't send email, so you pass it on.</p>
        )}
        {problem ? <p role="alert" className="text-body-sm text-danger">{problem}</p> : null}
      </div>
    </Dialog>
  );
}
