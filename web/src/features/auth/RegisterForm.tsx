import { useMutation } from "@apollo/client/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Field } from "glass-ux/field";
import { Input } from "glass-ux/input";
import { LoginFlow } from "glass-ux/login-flow";
import { useState } from "react";

import { Button } from "@/components/ui";
import { RegisterDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { errorMessage } from "@/lib/graphql/errors";

interface RegisterFormProps {
  /** First-run setup creates the administrator; no invite is needed. */
  isSetup: boolean;
  inviteToken?: string;
}

export function RegisterForm({ isSetup, inviteToken }: RegisterFormProps) {
  const navigate = useNavigate();
  const [register] = useMutation(RegisterDocument);
  const [pending, setPending] = useState(false);
  const [code, setCode] = useState(inviteToken ?? "");
  const [ready, setReady] = useState(isSetup || Boolean(inviteToken?.trim()));

  // LoginFlow owns the account fields. Invite collection is a host-owned step because
  // the public sign-up contract has no arbitrary-field slot.
  if (!ready) {
    return (
      <form onSubmit={(event) => { event.preventDefault(); setCode(code.trim()); setReady(true); }} className="flex flex-col gap-5">
        <h1 className="text-display-md text-foreground">Create your account</h1>
        <Field label="Invite code">
          <Input value={code} onValueChange={setCode} autoFocus autoComplete="off" className="font-mono" />
        </Field>
        <Button type="submit" variant="primary" fullWidth>Continue</Button>
        <p className="text-center text-body-sm text-muted">
          Already have an account? <Link to="/login" className="nav-focus rounded text-brand hover:underline">Sign in</Link>
        </p>
      </form>
    );
  }

  return (
    <LoginFlow
      surface={false}
      headingLevel={1}
      view="sign-up"
      autoFocus
      signUp={{ name: true, identifier: "email", confirmPassword: true, providers: false }}
      passwordPolicy={(password) => password.length < 10 ? "Use at least 10 characters" : undefined}
      labels={{ signUpTitle: isSetup ? "Set up Librarian" : "Create your account", signUpAction: isSetup ? "Create administrator" : "Create account" }}
      header={isSetup ? <p className="text-body-sm text-muted">This first account becomes the administrator.</p> : undefined}
      getViewHref={(view) => view === "sign-in" ? "/login" : undefined}
      onViewChange={(view) => { if (view === "sign-in") void navigate({ to: "/login" }); }}
      footer={!isSetup ? <Button variant="ghost" size="sm" isDisabled={pending} onPress={() => setReady(false)}>Change invite code</Button> : undefined}
      onSignUp={async (values) => {
        const name = values.name?.trim() ?? "";
        if (name.length > 80) return { status: "error", field: "name", message: "Use at most 80 characters" };
        setPending(true);
        try {
          const { data } = await register({ variables: { input: { name, email: values.email ?? "", password: values.password, inviteToken: isSetup ? null : code.trim() || null } } });
          const payload = data?.register;
          if (!payload?.success || !payload.user || !payload.tokens) {
            return { status: "error", message: payload?.error ?? "Couldn't create the account" };
          }
          session.establish(payload.user, payload.tokens.expiresIn);
          await navigate({ to: "/" });
        } catch (error) {
          return { status: "error", message: errorMessage(error, "Couldn't create the account") };
        } finally {
          setPending(false);
        }
      }}
    />
  );
}
