import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { Button, Card, CardBody, CardHeader, Input, Divider } from '@heroui/react'
import { useAuth } from '../../hooks/useAuth'

export const Route = createFileRoute('/auth/login')({
  component: LoginPage,
})

function LoginPage() {
  const navigate = useNavigate()
  const { signIn, signUp } = useAuth()
  const [isSignUp, setIsSignUp] = useState(false)
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    setLoading(true)

    try {
      if (isSignUp) {
        await signUp(email, password)
        alert('Check your email for a confirmation link!')
      } else {
        await signIn(email, password)
        navigate({ to: '/' })
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-[calc(100vh-4rem)] flex items-center justify-center px-4">
      <div className="w-full max-w-md">
        <Card className="bg-content1">
          <CardHeader className="flex flex-col gap-1 items-center pb-0">
            <h1 className="text-2xl font-bold">
              {isSignUp ? 'Create Account' : 'Sign In'}
            </h1>
            <p className="text-small text-default-500">
              {isSignUp
                ? 'Create your account to get started'
                : 'Welcome back! Sign in to continue'}
            </p>
          </CardHeader>
          <Divider className="my-4" />
          <CardBody>
            <form onSubmit={handleSubmit} className="flex flex-col gap-4">
              <Input
                type="email"
                label="Email"
                placeholder="you@example.com"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                isRequired
                variant="flat"
                classNames={{
                  inputWrapper: 'bg-default-100',
                  input: 'text-foreground',
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
                variant="flat"
                classNames={{
                  inputWrapper: 'bg-default-100',
                  input: 'text-foreground',
                }}
              />

              {error && (
                <Card className="bg-danger-50 border-danger">
                  <CardBody className="py-2 px-3">
                    <p className="text-danger text-sm">{error}</p>
                  </CardBody>
                </Card>
              )}

              <Button
                type="submit"
                color="primary"
                size="lg"
                isLoading={loading}
                className="w-full font-semibold"
              >
                {isSignUp ? 'Create Account' : 'Sign In'}
              </Button>
            </form>

            <Divider className="my-4" />

            <div className="text-center">
              <Button
                variant="light"
                color="primary"
                onPress={() => setIsSignUp(!isSignUp)}
              >
                {isSignUp
                  ? 'Already have an account? Sign in'
                  : "Don't have an account? Sign up"}
              </Button>
            </div>
          </CardBody>
        </Card>
      </div>
    </div>
  )
}
