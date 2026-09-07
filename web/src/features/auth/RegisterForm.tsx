import { useMutation } from "@apollo/client/react";
import { Alert } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useForm } from "react-hook-form";

import { Button, FieldGroup, FormTextField } from "@/components/ui";
import { RegisterDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { errorMessage } from "@/lib/graphql/errors";

import { registerSchema, type RegisterValues } from "./schemas";

interface RegisterFormProps {
  /** First-run setup creates the administrator; no invite is needed. */
  isSetup: boolean;
  inviteToken?: string;
}

export function RegisterForm({ isSetup, inviteToken }: RegisterFormProps) {
  const navigate = useNavigate();
  const [register, { loading }] = useMutation(RegisterDocument);
  const [failure, setFailure] = useState<string | null>(null);
  const form = useForm<RegisterValues>({
    resolver: zodResolver(registerSchema),
    defaultValues: { name: "", email: "", password: "", confirm: "", inviteToken: inviteToken ?? "" },
  });

  const submit = form.handleSubmit(async (values) => {
    setFailure(null);
    try {
      const { data } = await register({
        variables: { input: { name: values.name, email: values.email, password: values.password, inviteToken: values.inviteToken || null } },
      });
      const payload = data?.register;
      if (!payload?.success || !payload.user || !payload.tokens) {
        setFailure(payload?.error ?? "Couldn't create the account");
        return;
      }
      session.establish(payload.user, payload.tokens.expiresIn);
      await navigate({ to: "/" });
    } catch (error) {
      setFailure(errorMessage(error, "Couldn't create the account"));
    }
  });

  return (
    <form onSubmit={submit} noValidate className="flex flex-col gap-5">
      <div>
        <h1 className="text-display-md text-foreground">{isSetup ? "Set up Librarian" : "Create your account"}</h1>
        {isSetup ? <p className="mt-1 text-body-sm text-muted">This first account becomes the administrator.</p> : null}
      </div>
      {failure ? (
        <Alert status="danger">
          <Alert.Content>
            <Alert.Description>{failure}</Alert.Description>
          </Alert.Content>
        </Alert>
      ) : null}
      <FieldGroup>
        <FormTextField control={form.control} name="name" label="Name" autoComplete="name" autoFocus isRequired />
        <FormTextField control={form.control} name="email" label="Email" type="email" autoComplete="email" isRequired />
        <FormTextField control={form.control} name="password" label="Password" type="password" autoComplete="new-password" isRequired />
        <FormTextField control={form.control} name="confirm" label="Confirm password" type="password" autoComplete="new-password" isRequired />
        {!isSetup ? <FormTextField control={form.control} name="inviteToken" label="Invite code" mono /> : null}
      </FieldGroup>
      <Button type="submit" variant="primary" size="lg" fullWidth isPending={loading} className="mt-1">
        {isSetup ? "Create administrator" : "Create account"}
      </Button>
      {!isSetup ? (
        <p className="text-center text-body-sm text-muted">
          Already have an account?{" "}
          <Link to="/login" className="nav-focus rounded text-brand hover:underline">
            Sign in
          </Link>
        </p>
      ) : null}
    </form>
  );
}
