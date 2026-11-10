import { useMutation } from "@apollo/client/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { LoginFlow } from "glass-ux/login-flow";
import { otpSchema } from "glass-ux/otp-field";
import { passwordSchema } from "glass-ux/password-field";
import { useState } from "react";
import { z } from "zod";

import { Button } from "@/components/ui";
import { CompletePasswordChangeDocument, CompleteTwoFactorSignInDocument, LoginDocument, type SignedInFieldsFragment } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { errorMessage } from "@/lib/graphql/errors";

interface LoginFormProps {
  redirectTo?: string;
  allowRegister: boolean;
}

/** What a correct password led to: a session, another step, or an error. */
type Step = { stage: "two_factor" | "password_change"; token: string };

const totpForm = z.object({ code: otpSchema({ length: 6 }) });
const recoveryForm = z.object({ code: otpSchema({ length: 10, characters: "alphanumeric" }) });
const newPasswordForm = z.object({ password: passwordSchema({ minLength: 10, required: true }) });

/** The second step: six digits from the authenticator app, or a recovery code once. */
function TwoFactorStep({ onCode, onBack }: { onCode: (code: string) => Promise<void>; onBack: () => void }) {
  const [recovery, setRecovery] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: { code: "" },
    validators: { onDynamic: recovery ? recoveryForm : totpForm },
    onSubmit: async ({ value, formApi }) => {
      setError(null);
      try {
        await onCode(value.code);
      } catch (failure) {
        formApi.reset({ code: "" });
        throw failure;
      }
    },
  });
  return (
    <div className="flex min-w-0 flex-col gap-4">
      <h1 className="text-title text-foreground">Two-factor sign-in</h1>
      <p className="text-body-sm text-muted">{recovery ? "Enter one of your recovery codes. Each works once." : "Enter the 6-digit code from your authenticator app."}</p>
      {error ? <p role="alert" className="text-body-sm text-danger">{error}</p> : null}
      <Form form={form} onError={(failure) => setError(errorMessage(failure, "That code didn't work"))}>
        <form.AppField name="code" key={recovery ? "recovery" : "totp"}>
          {(field) => (recovery
            ? <field.OtpField label="Recovery code" length={10} characters="alphanumeric" groups={[5, 5]} required />
            : <field.OtpField label="Code" length={6} autoSubmit required />)}
        </form.AppField>
        <FormActions>
          <Button variant="ghost" onPress={onBack}>Back</Button>
          <form.SubmitButton>Sign in</form.SubmitButton>
        </FormActions>
      </Form>
      <Button variant="ghost" size="sm" className="self-start" onPress={() => { setRecovery((value) => !value); form.reset({ code: "" }); }}>
        {recovery ? "Use the authenticator app" : "Use a recovery code"}
      </Button>
    </div>
  );
}

/** After a temporary password: choose your own before you are signed in. */
function NewPasswordStep({ onPassword, onBack }: { onPassword: (password: string) => Promise<void>; onBack: () => void }) {
  const [error, setError] = useState<string | null>(null);
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: { password: "" },
    validators: { onDynamic: newPasswordForm },
    onSubmit: async ({ value }) => onPassword(value.password),
  });
  return (
    <div className="flex min-w-0 flex-col gap-4">
      <h1 className="text-title text-foreground">Choose a new password</h1>
      <p className="text-body-sm text-muted">Your password was set by an administrator. Choose your own to finish signing in.</p>
      {error ? <p role="alert" className="text-body-sm text-danger">{error}</p> : null}
      <Form form={form} onError={(failure) => setError(errorMessage(failure, "The password could not be saved"))}>
        <form.AppField name="password">{(field) => <field.PasswordField label="New password" purpose="new" required />}</form.AppField>
        <FormActions>
          <Button variant="ghost" onPress={onBack}>Back</Button>
          <form.SubmitButton>Save and sign in</form.SubmitButton>
        </FormActions>
      </Form>
    </div>
  );
}

export function LoginForm({ redirectTo, allowRegister }: LoginFormProps) {
  const navigate = useNavigate();
  const [login] = useMutation(LoginDocument);
  const [completeTwoFactor] = useMutation(CompleteTwoFactorSignInDocument);
  const [completePassword] = useMutation(CompletePasswordChangeDocument);
  const [step, setStep] = useState<Step | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const destination = redirectTo?.startsWith("/") && !redirectTo.startsWith("//") ? redirectTo : "/";

  /** A session signs in; a challenge moves on to its step. Anything else is the error to show. */
  const settle = async (payload: Omit<SignedInFieldsFragment, "challenge"> & { challenge?: SignedInFieldsFragment["challenge"] } | null | undefined): Promise<string | null> => {
    if (payload?.challenge && (payload.challenge.stage === "two_factor" || payload.challenge.stage === "password_change")) {
      setProblem(null);
      setStep({ stage: payload.challenge.stage, token: payload.challenge.token });
      return null;
    }
    if (!payload?.success || !payload.user || !payload.tokens) return payload?.error ?? "Sign in failed";
    session.establish(payload.user, payload.tokens.expiresIn);
    await navigate({ href: destination });
    return null;
  };
  const restart = () => { setStep(null); setProblem(null); };
  // A wrong code keeps the step; an expired sign-in goes back to the password.
  const stepFailed = (message: string) => {
    if (message.includes("expired")) { setStep(null); setProblem(message); }
    throw new Error(message);
  };

  if (step) {
    return (
      <div className="flex min-w-0 flex-col gap-3" aria-live="polite">
        {step.stage === "two_factor" ? (
          <TwoFactorStep
            onBack={restart}
            onCode={async (code) => {
              const failure = await settle((await completeTwoFactor({ variables: { input: { token: step.token, code } } })).data?.completeTwoFactorSignIn);
              if (failure) stepFailed(failure);
            }}
          />
        ) : (
          <NewPasswordStep
            onBack={restart}
            onPassword={async (password) => {
              const failure = await settle((await completePassword({ variables: { input: { token: step.token, newPassword: password } } })).data?.completePasswordChange);
              if (failure) stepFailed(failure);
            }}
          />
        )}
      </div>
    );
  }

  // A plain identifier preserves the backend's nonempty-only sign-in contract.
  return (
    <div className="flex min-w-0 flex-col gap-3">
      {problem ? <p role="alert" className="text-body-sm text-danger">{problem}</p> : null}
      <LoginFlow
        surface={false}
        headingLevel={1}
        identifier="username"
        labels={{ usernameLabel: "Username or email", identifierRequired: () => "Enter your username or email" }}
        autoFocus
        onSignIn={async ({ identifier, password }) => {
          try {
            const { data } = await login({ variables: { input: { usernameOrEmail: identifier, password } } });
            const failure = await settle(data?.login);
            if (failure) return { status: "error", message: failure };
          } catch (error) {
            return { status: "error", message: errorMessage(error, "Sign in failed") };
          }
        }}
        footer={allowRegister ? (
          <p className="text-center text-body-sm text-muted">
            Have an invite?{" "}
            <Link to="/register" className="nav-focus rounded text-brand hover:underline">Create an account</Link>
          </p>
        ) : undefined}
      />
    </div>
  );
}
