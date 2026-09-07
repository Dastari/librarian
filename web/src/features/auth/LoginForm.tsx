import { useMutation } from "@apollo/client/react";
import { Alert } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useForm } from "react-hook-form";

import { Button, FieldGroup, FormTextField } from "@/components/ui";
import { LoginDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { errorMessage } from "@/lib/graphql/errors";

import { loginSchema, type LoginValues } from "./schemas";

interface LoginFormProps {
  redirectTo?: string;
  allowRegister: boolean;
}

export function LoginForm({ redirectTo, allowRegister }: LoginFormProps) {
  const navigate = useNavigate();
  const [login, { loading }] = useMutation(LoginDocument);
  const [failure, setFailure] = useState<string | null>(null);
  const form = useForm<LoginValues>({ resolver: zodResolver(loginSchema), defaultValues: { usernameOrEmail: "", password: "" } });

  const submit = form.handleSubmit(async (values) => {
    setFailure(null);
    try {
      const { data } = await login({ variables: { input: values } });
      const payload = data?.login;
      if (!payload?.success || !payload.user || !payload.tokens) {
        setFailure(payload?.error ?? "Sign in failed");
        return;
      }
      session.establish(payload.user, payload.tokens.expiresIn);
      await navigate({ href: redirectTo && redirectTo.startsWith("/") ? redirectTo : "/" });
    } catch (error) {
      setFailure(errorMessage(error, "Sign in failed"));
    }
  });

  return (
    <form onSubmit={submit} noValidate className="flex flex-col gap-5">
      <h1 className="text-display-md text-foreground">Sign in</h1>
      {failure ? (
        <Alert status="danger">
          <Alert.Content>
            <Alert.Description>{failure}</Alert.Description>
          </Alert.Content>
        </Alert>
      ) : null}
      <FieldGroup>
        <FormTextField control={form.control} name="usernameOrEmail" label="Username or email" autoComplete="username" autoFocus isRequired />
        <FormTextField control={form.control} name="password" label="Password" type="password" autoComplete="current-password" isRequired />
      </FieldGroup>
      <Button type="submit" variant="primary" size="lg" fullWidth isPending={loading} className="mt-1">
        Sign in
      </Button>
      {allowRegister ? (
        <p className="text-center text-body-sm text-muted">
          Have an invite?{" "}
          <Link to="/register" className="nav-focus rounded text-brand hover:underline">
            Create an account
          </Link>
        </p>
      ) : null}
    </form>
  );
}
