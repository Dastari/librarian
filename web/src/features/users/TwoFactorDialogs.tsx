import { useMutation } from "@apollo/client/react";
import { CopyButton } from "glass-ux/copy-button";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { otpSchema } from "glass-ux/otp-field";
import { currentPasswordSchema } from "glass-ux/password-field";
import { Surface } from "glass-ux/surface";
import { toast } from "glass-ux/toast";
import { useState } from "react";
import { z } from "zod";

import { Button, Dialog } from "@/components/ui";
import { BeginTwoFactorSetupDocument, ConfirmTwoFactorSetupDocument, DisableTwoFactorDocument, RegenerateRecoveryCodesDocument } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

const passwordForm = z.object({ password: currentPasswordSchema() });
const codeForm = z.object({ code: otpSchema({ length: 6 }) });

/** The QR code the server drew, as an image (never inserted as markup). */
export function qrImageSource(svg: string): string {
  return `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(svg)))}`;
}

/** "abcde-fghjk" in groups, for reading aloud and copying. */
export function RecoveryCodes({ codes }: { codes: readonly string[] }) {
  const text = codes.join("\n");
  return (
    <div className="flex min-w-0 flex-col gap-3">
      <Surface material="inset" padding="tile" radius="tile">
        <ol aria-label="Recovery codes" className="grid min-w-0 grid-cols-2 gap-x-6 gap-y-1 font-mono text-body">
          {codes.map((code) => <li key={code}>{code}</li>)}
        </ol>
      </Surface>
      <div className="flex flex-wrap items-center gap-2">
        <CopyButton value={text} label="Copy codes" showLabel />
        <Button
          variant="ghost"
          size="sm"
          onPress={() => {
            const url = URL.createObjectURL(new Blob([`Librarian recovery codes\n\n${text}\n`], { type: "text/plain" }));
            const anchor = Object.assign(document.createElement("a"), { href: url, download: "librarian-recovery-codes.txt" });
            anchor.click();
            URL.revokeObjectURL(url);
          }}
        >
          Download
        </Button>
      </div>
      <p className="text-body-sm text-muted">Each code signs you in once if you lose your authenticator. Keep them somewhere safe: they are not shown again.</p>
    </div>
  );
}

type Setup = { secret: string; qrSvg: string };

/**
 * Turning on two-factor sign-in (design.md Q147): confirm your password, scan the QR code or type
 * the key into an authenticator app, confirm with a code from it, then keep the recovery codes.
 */
export function TwoFactorSetupDialog({ open, onOpenChange, onChanged }: { open: boolean; onOpenChange: (open: boolean) => void; onChanged?: () => void }) {
  const [begin] = useMutation(BeginTwoFactorSetupDocument);
  const [confirm] = useMutation(ConfirmTwoFactorSetupDocument);
  const [setup, setSetup] = useState<Setup | null>(null);
  const [codes, setCodes] = useState<string[] | null>(null);
  const passwordStep = useAppForm({
    ...formBehaviour,
    defaultValues: { password: "" },
    validators: { onDynamic: passwordForm },
    onSubmit: async ({ value }) => {
      const result = assertSuccess((await begin({ variables: { input: { password: value.password } } })).data?.beginTwoFactorSetup, "Two-factor sign-in could not be started");
      if (!result.secret || !result.qrSvg) throw new Error("Two-factor sign-in could not be started");
      setSetup({ secret: result.secret, qrSvg: result.qrSvg });
    },
  });
  const codeStep = useAppForm({
    ...formBehaviour,
    defaultValues: { code: "" },
    validators: { onDynamic: codeForm },
    onSubmit: async ({ value, formApi }) => {
      try {
        const result = assertSuccess((await confirm({ variables: { input: { code: value.code } } })).data?.confirmTwoFactorSetup, "That code didn't work");
        setCodes(result.recoveryCodes);
        onChanged?.();
      } catch (error) {
        formApi.reset({ code: "" });
        throw error;
      }
    },
  });
  const close = () => onOpenChange(false);
  const title = codes ? "Save your recovery codes" : setup ? "Scan with your authenticator app" : "Turn on two-factor sign-in";

  return (
    <Dialog isOpen={open} onOpenChange={onOpenChange} title={title} size="sm" footer={codes ? <Button variant="primary" onPress={() => { toast.success("Two-factor sign-in is on"); close(); }}>I've saved them</Button> : undefined}>
      {codes ? (
        <RecoveryCodes codes={codes} />
      ) : setup ? (
        <Form form={codeStep} onError={(error) => toast.danger(errorMessage(error, "That code didn't work"))}>
          <div className="flex min-w-0 flex-col items-center gap-3">
            <img src={qrImageSource(setup.qrSvg)} alt="QR code for your authenticator app" width={200} height={200} className="rounded-[var(--radius-control)] bg-white p-2" />
            <div className="flex w-full min-w-0 flex-col gap-1">
              <p className="text-label-sm text-muted">Or type this key</p>
              <Surface material="inset" padding="tile" radius="tile" className="flex min-w-0 items-center gap-2">
                <code className="min-w-0 flex-1 font-mono text-body-sm [overflow-wrap:anywhere]">{setup.secret.match(/.{1,4}/g)?.join(" ")}</code>
                <CopyButton value={setup.secret} label="Copy key" />
              </Surface>
            </div>
          </div>
          <codeStep.AppField name="code">{(field) => <field.OtpField label="Code from the app" length={6} autoSubmit required />}</codeStep.AppField>
          <FormActions>
            <codeStep.CancelButton onCancel={close}>Cancel</codeStep.CancelButton>
            <codeStep.SubmitButton>Turn on</codeStep.SubmitButton>
          </FormActions>
        </Form>
      ) : (
        <Form form={passwordStep} onError={(error) => toast.danger(errorMessage(error, "Two-factor sign-in could not be started"))}>
          <p className="text-body-sm text-muted">Signing in will also ask for a code from an app such as 1Password, Google Authenticator or Authy.</p>
          <passwordStep.AppField name="password">{(field) => <field.PasswordField label="Your password" purpose="current" required />}</passwordStep.AppField>
          <FormActions>
            <passwordStep.CancelButton onCancel={close}>Cancel</passwordStep.CancelButton>
            <passwordStep.SubmitButton>Continue</passwordStep.SubmitButton>
          </FormActions>
        </Form>
      )}
    </Dialog>
  );
}

/** Turn two-factor sign-in off, or make new recovery codes: both need your password. */
export function TwoFactorPasswordDialog({ purpose, open, onOpenChange, onChanged }: { purpose: "disable" | "codes"; open: boolean; onOpenChange: (open: boolean) => void; onChanged?: () => void }) {
  const [disable] = useMutation(DisableTwoFactorDocument);
  const [regenerate] = useMutation(RegenerateRecoveryCodesDocument);
  const [codes, setCodes] = useState<string[] | null>(null);
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: { password: "" },
    validators: { onDynamic: passwordForm },
    onSubmit: async ({ value }) => {
      if (purpose === "disable") {
        assertSuccess((await disable({ variables: { input: { password: value.password } } })).data?.disableTwoFactor, "Two-factor sign-in could not be turned off");
        toast.success("Two-factor sign-in is off");
        onChanged?.();
        onOpenChange(false);
      } else {
        const result = assertSuccess((await regenerate({ variables: { input: { password: value.password } } })).data?.regenerateRecoveryCodes, "New codes could not be made");
        setCodes(result.recoveryCodes);
        onChanged?.();
      }
    },
  });
  return (
    <Dialog isOpen={open} onOpenChange={onOpenChange} title={codes ? "Your new recovery codes" : purpose === "disable" ? "Turn off two-factor sign-in" : "New recovery codes"} size="sm"
      footer={codes ? <Button variant="primary" onPress={() => onOpenChange(false)}>I've saved them</Button> : undefined}>
      {codes ? <RecoveryCodes codes={codes} /> : (
        <Form form={form} onError={(error) => toast.danger(errorMessage(error))}>
          <p className="text-body-sm text-muted">{purpose === "disable" ? "Signing in will ask for your password only. Your recovery codes stop working." : "Your current recovery codes stop working."}</p>
          <form.AppField name="password">{(field) => <field.PasswordField label="Your password" purpose="current" required />}</form.AppField>
          <FormActions>
            <form.CancelButton onCancel={() => onOpenChange(false)}>Cancel</form.CancelButton>
            <form.SubmitButton>{purpose === "disable" ? "Turn off" : "Make new codes"}</form.SubmitButton>
          </FormActions>
        </Form>
      )}
    </Dialog>
  );
}
