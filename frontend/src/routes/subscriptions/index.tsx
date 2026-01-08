import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState } from 'react'
import {
  Button,
  Card,
  CardBody,
  CardHeader,
  Input,
  Chip,
  Divider,
} from '@heroui/react'
import type { Subscription } from '../../lib/api'

export const Route = createFileRoute('/subscriptions/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/auth/login' })
    }
  },
  component: SubscriptionsPage,
})

// Mock data for display
const mockSubscriptions: Subscription[] = [
  {
    id: '1',
    show_name: 'Example Show',
    tvdb_id: 12345,
    quality_profile_id: 'default',
    monitored: true,
  },
]

function SubscriptionCard({ subscription }: { subscription: Subscription }) {
  return (
    <Card className="bg-content1">
      <CardBody>
        <div className="flex items-start justify-between mb-3">
          <div>
            <h3 className="font-semibold text-foreground">{subscription.show_name}</h3>
            <p className="text-default-500 text-sm">TVDB: {subscription.tvdb_id}</p>
          </div>
          <Chip
            size="sm"
            color={subscription.monitored ? 'success' : 'default'}
            variant="flat"
          >
            {subscription.monitored ? 'Monitored' : 'Not Monitored'}
          </Chip>
        </div>

        <div className="flex gap-2">
          <Button size="sm" variant="flat" className="flex-1">
            Search
          </Button>
          <Button size="sm" variant="flat" isIconOnly>
            ⚙️
          </Button>
        </div>
      </CardBody>
    </Card>
  )
}

function SubscriptionsPage() {
  const [searchQuery, setSearchQuery] = useState('')

  const filteredSubscriptions = mockSubscriptions.filter((sub) =>
    sub.show_name.toLowerCase().includes(searchQuery.toLowerCase())
  )

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Subscriptions</h1>
        <Button color="primary">+ Add Show</Button>
      </div>

      {/* Search */}
      <div className="mb-8">
        <Input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search subscriptions..."
          className="max-w-md"
          variant="bordered"
          isClearable
          onClear={() => setSearchQuery('')}
        />
      </div>

      {/* Subscriptions grid */}
      {filteredSubscriptions.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredSubscriptions.map((sub) => (
            <SubscriptionCard key={sub.id} subscription={sub} />
          ))}
        </div>
      ) : mockSubscriptions.length === 0 ? (
        <Card className="bg-content1">
          <CardBody className="text-center py-12">
            <span className="text-5xl mb-4 block">📺</span>
            <p className="text-lg text-default-600 mb-2">No subscriptions yet</p>
            <p className="text-sm text-default-400 mb-4">
              Add shows to automatically download new episodes.
            </p>
            <Button color="primary">Add Your First Show</Button>
          </CardBody>
        </Card>
      ) : (
        <Card className="bg-content1">
          <CardBody className="text-center py-8">
            <p className="text-default-500">No subscriptions match your search.</p>
          </CardBody>
        </Card>
      )}

      {/* Info box */}
      <Card className="mt-8 bg-content2">
        <CardHeader>
          <h3 className="font-semibold">How Subscriptions Work</h3>
        </CardHeader>
        <Divider />
        <CardBody>
          <ul className="text-default-500 text-sm space-y-1">
            <li>• Add a TV show to monitor for new episodes</li>
            <li>• Set a quality profile (e.g., 1080p, x265)</li>
            <li>• Librarian automatically searches indexers for new releases</li>
            <li>• Downloads are added and organized in your library</li>
          </ul>
        </CardBody>
      </Card>
    </div>
  )
}
