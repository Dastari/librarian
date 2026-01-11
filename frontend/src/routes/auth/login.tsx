import { createFileRoute, redirect } from '@tanstack/react-router'

// Search params for the login page
interface LoginSearchParams {
  redirect?: string
}

// The login page now redirects to the home page with the sign-in modal open
// This provides a consistent UX where users always see the modal overlay
export const Route = createFileRoute('/auth/login')({
  validateSearch: (search: Record<string, unknown>): LoginSearchParams => {
    return {
      redirect: typeof search.redirect === 'string' ? search.redirect : undefined,
    }
  },
  beforeLoad: ({ search }) => {
    // Redirect to home page with sign-in modal open
    throw redirect({
      to: '/',
      search: {
        signin: true,
        redirect: search.redirect,
      },
    })
  },
  component: () => null, // Never rendered due to redirect
})
