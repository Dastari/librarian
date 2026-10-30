import { useMutation } from "@apollo/client/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { LoginFlow } from "glass-ux/login-flow";

import { LoginDocument } from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { errorMessage } from "@/lib/graphql/errors";

interface LoginFormProps {
  redirectTo?: string;
  allowRegister: boolean;
}

export function LoginForm({ redirectTo, allowRegister }: LoginFormProps) {
  const navigate = useNavigate();
  const [login] = useMutation(LoginDocument);

  // A plain identifier preserves the backend's nonempty-only sign-in contract.
  return (
    <LoginFlow
      surface={false}
      headingLevel={1}
      identifier="username"
      labels={{ usernameLabel: "Username or email", identifierRequired: () => "Enter your username or email" }}
      autoFocus
      onSignIn={async ({ identifier, password }) => {
        try {
          const { data } = await login({ variables: { input: { usernameOrEmail: identifier, password } } });
          const payload = data?.login;
          if (!payload?.success || !payload.user || !payload.tokens) {
            return { status: "error", message: payload?.error ?? "Sign in failed" };
          }
          session.establish(payload.user, payload.tokens.expiresIn);
          await navigate({ href: redirectTo?.startsWith("/") && !redirectTo.startsWith("//") ? redirectTo : "/" });
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
  );
}
