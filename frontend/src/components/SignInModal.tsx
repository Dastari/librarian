import { useState, useEffect } from "react";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Button } from "@heroui/button";
import { Input } from "@heroui/input";
import { Spinner } from "@heroui/spinner";
import { addToast } from "@heroui/toast";
import { IconShieldCheck, IconAlertCircle } from "@tabler/icons-react";
import { useAuth } from "../hooks/useAuth";
import { graphqlClient, NEEDS_SETUP_QUERY } from "../lib/graphql";

interface SignInModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess?: () => void;
  redirectUrl?: string;
}

export function SignInModal({
  isOpen,
  onClose,
  onSuccess,
  redirectUrl,
}: SignInModalProps) {
  const { signIn, signUp } = useAuth();
  const [isSignUp, setIsSignUp] = useState(false);
  const [needsSetup, setNeedsSetup] = useState<boolean | null>(null);
  const [checkingSetup, setCheckingSetup] = useState(true);

  // Form fields
  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");

  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  // Check if setup is needed when modal opens
  useEffect(() => {
    if (isOpen) {
      checkSetupStatus();
    }
  }, [isOpen]);

  const checkSetupStatus = async () => {
    setCheckingSetup(true);
    try {
      const result = await graphqlClient
        .query<{ needsSetup: boolean }>(NEEDS_SETUP_QUERY, {})
        .toPromise();

      if (result.data) {
        setNeedsSetup(result.data.needsSetup);
        // If setup is needed, force sign-up mode
        if (result.data.needsSetup) {
          setIsSignUp(true);
        }
      }
    } catch (err) {
      console.error("[SignInModal] Failed to check setup status:", err);
      setNeedsSetup(false);
    } finally {
      setCheckingSetup(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);

    try {
      if (isSignUp) {
        // Validate email is provided
        if (!email.trim()) {
          setError("Email is required");
          setLoading(false);
          return;
        }

        // Validate name is provided
        if (!name.trim()) {
          setError("Full name is required");
          setLoading(false);
          return;
        }

        // Validate password length
        if (password.length < 6) {
          setError("Password must be at least 6 characters");
          setLoading(false);
          return;
        }

        await signUp(email.trim(), name.trim(), password);

        addToast({
          title: needsSetup ? "Admin Account Created" : "Account Created",
          description: needsSetup
            ? "Welcome! Your admin account has been set up."
            : "Welcome! Your account has been created.",
          color: "success",
        });
        handleClose();

        if (redirectUrl) {
          window.location.href = redirectUrl;
        } else if (onSuccess) {
          onSuccess();
        }
      } else {
        // Validate email is provided
        if (!email.trim()) {
          setError("Email is required");
          setLoading(false);
          return;
        }

        await signIn(email.trim(), password);
        handleClose();

        if (redirectUrl) {
          window.location.href = redirectUrl;
        } else if (onSuccess) {
          onSuccess();
        }
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : "An error occurred";
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    // Reset form state when closing
    setEmail("");
    setName("");
    setPassword("");
    setError("");
    setIsSignUp(false);
    setNeedsSetup(null);
    setCheckingSetup(true);
    onClose();
  };

  // Show loading while checking setup status
  if (checkingSetup) {
    return (
      <Modal
        isOpen={isOpen}
        onClose={handleClose}
        placement="center"
        backdrop="blur"
        classNames={{
          backdrop: "bg-black/60",
        }}
      >
        <ModalContent>
          <ModalBody className="py-12 flex flex-col items-center gap-4">
            <Spinner size="lg" />
            <p className="text-default-500">Checking setup status...</p>
          </ModalBody>
        </ModalContent>
      </Modal>
    );
  }

  const showAdminSetup = needsSetup && isSignUp;
  const heading = showAdminSetup
    ? "Create Admin Account"
    : isSignUp
      ? "Sign Up"
      : "Sign In";

  return (
    <Modal
      isOpen={isOpen}
      onClose={handleClose}
      placement="center"
      backdrop="blur"
      classNames={{
        backdrop: "bg-black/60",
      }}
    >
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1 items-center pt-6">
          {/* Logo and brand */}
          <div className="flex flex-row items-center gap-2 mb-2">
            <img src="/logo.svg" alt="" className="h-12 w-12" />
            <span
              className="text-2xl"
              style={{ fontFamily: '"Playwrite Australia SA", cursive' }}
            >
              Librarian
            </span>
          </div>

          <p className="text-lg font-semibold">{heading}</p>

          {showAdminSetup ? (
            <div className="flex flex-col items-center gap-2 mt-2">
              <div className="flex items-center gap-2 text-primary">
                <IconShieldCheck size={20} />
                <span className="text-sm font-medium">First-time Setup</span>
              </div>
              <p className="text-small text-default-500 font-normal text-center max-w-xs">
                This is the first account and will have administrator
                privileges.
              </p>
            </div>
          ) : (
            <p className="text-small text-default-500 font-normal">
              {isSignUp
                ? "Create your account to get started"
                : "Welcome back! Sign in to continue"}
            </p>
          )}
        </ModalHeader>
        <ModalBody>
          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            {isSignUp ? (
              <>
                {/* Sign Up form */}
                <Input
                  type="email"
                  label="Email"
                  placeholder="you@example.com"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  isRequired
                  autoComplete="email"
                  variant="flat"
                  classNames={{
                    inputWrapper: "bg-default-100",
                    input: "text-foreground",
                  }}
                />

                <Input
                  type="text"
                  label="Full Name"
                  placeholder="Enter your name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  isRequired
                  autoComplete="name"
                  variant="flat"
                  classNames={{
                    inputWrapper: "bg-default-100",
                    input: "text-foreground",
                  }}
                />

                <div className="flex flex-col gap-1">
                  <Input
                    type="password"
                    label="Password"
                    placeholder="••••••••"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    isRequired
                    minLength={6}
                    autoComplete="new-password"
                    variant="flat"
                    classNames={{
                      inputWrapper: "bg-default-100",
                      input: "text-foreground",
                    }}
                  />
                  <p className="text-tiny text-default-400 pl-1">
                    Minimum 6 characters
                  </p>
                </div>
              </>
            ) : (
              <>
                {/* Sign In form */}
                <Input
                  type="email"
                  label="Email"
                  placeholder="you@example.com"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  isRequired
                  autoComplete="email"
                  variant="flat"
                  classNames={{
                    inputWrapper: "bg-default-100",
                    input: "text-foreground",
                  }}
                />

                <Input
                  type="password"
                  label="Password"
                  placeholder="••••••••"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  isRequired
                  minLength={6}
                  autoComplete="current-password"
                  variant="flat"
                  classNames={{
                    inputWrapper: "bg-default-100",
                    input: "text-foreground",
                  }}
                />
              </>
            )}

            {error && (
              <div className="flex items-start gap-2 p-3 rounded-lg bg-danger-50/50 border border-danger-200">
                <IconAlertCircle
                  size={18}
                  className="text-danger flex-shrink-0 mt-0.5"
                />
                <p className="text-sm text-danger">{error}</p>
              </div>
            )}

            <Button
              type="submit"
              color="primary"
              size="lg"
              isLoading={loading}
              className="w-full font-semibold"
            >
              {showAdminSetup
                ? "Create Admin Account"
                : isSignUp
                  ? "Create Account"
                  : "Sign In"}
            </Button>
          </form>
        </ModalBody>

        {/* Only show toggle if not in setup mode */}
        {!needsSetup && (
          <ModalFooter className="justify-center">
            <Button
              variant="light"
              color="primary"
              onPress={() => {
                setIsSignUp(!isSignUp);
                setError("");
              }}
            >
              {isSignUp
                ? "Already have an account? Sign in"
                : "Don't have an account? Sign up"}
            </Button>
          </ModalFooter>
        )}
      </ModalContent>
    </Modal>
  );
}
