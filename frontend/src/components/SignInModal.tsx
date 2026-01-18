import { useState } from 'react'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { Input } from '@heroui/input'
import { Divider } from '@heroui/divider'
import { addToast } from '@heroui/toast'
import { useAuth } from '../hooks/useAuth'
import { InlineError } from './shared'

interface SignInModalProps {
  isOpen: boolean
  onClose: () => void
  onSuccess?: () => void
  redirectUrl?: string
}

export function SignInModal({ isOpen, onClose, onSuccess, redirectUrl }: SignInModalProps) {
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
        addToast({
          title: 'Account Created',
          description: 'Check your email for a confirmation link!',
          color: 'success',
        })
        onClose()
      } else {
        await signIn(email, password)
        onClose()
        // Handle redirect after successful sign in
        if (redirectUrl) {
          window.location.href = redirectUrl
        } else if (onSuccess) {
          onSuccess()
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred')
    } finally {
      setLoading(false)
    }
  }

  const handleClose = () => {
    // Reset form state when closing
    setEmail('')
    setPassword('')
    setError('')
    setIsSignUp(false)
    onClose()
  }

  return (
    <Modal
      isOpen={isOpen}
      onClose={handleClose}
      placement="center"
      backdrop="blur"
      classNames={{
        backdrop: 'bg-black/60',
      }}
    >
      <ModalContent>
        <ModalHeader className="flex flex-col gap-1 items-center">
          <h2 className="text-2xl font-bold">
            {isSignUp ? 'Create Account' : 'Sign In'}
          </h2>
          <p className="text-small text-default-500 font-normal">
            {isSignUp
              ? 'Create your account to get started'
              : 'Welcome back! Sign in to continue'}
          </p>
        </ModalHeader>
        <Divider />
        <ModalBody>
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

            {error && <InlineError message={error} />}

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
        </ModalBody>
        <Divider />
        <ModalFooter className="justify-center">
          <Button
            variant="light"
            color="primary"
            onPress={() => setIsSignUp(!isSignUp)}
          >
            {isSignUp
              ? 'Already have an account? Sign in'
              : "Don't have an account? Sign up"}
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
