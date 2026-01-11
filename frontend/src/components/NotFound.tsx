import { Link } from '@tanstack/react-router'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'

export function NotFound() {
  return (
    <div className="min-h-[calc(100vh-4rem)] flex items-center justify-center p-4">
      <Card className="max-w-md w-full">
        <CardBody className="text-center py-12">
          <div className="text-8xl mb-4">404</div>
          <h1 className="text-2xl font-bold mb-2">Page Not Found</h1>
          <p className="text-default-500 mb-6">
            The page you're looking for doesn't exist or has been moved.
          </p>
          <Link to="/">
            <Button color="primary" size="lg">
              Go Home
            </Button>
          </Link>
        </CardBody>
      </Card>
    </div>
  )
}
