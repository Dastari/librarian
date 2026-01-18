import { createFileRoute } from '@tanstack/react-router'
import { Card, CardBody } from '@heroui/card'
import { IconSettings } from '@tabler/icons-react'

export const Route = createFileRoute('/settings/')({
  component: GeneralSettingsPage,
})

function GeneralSettingsPage() {
  return (
    <div className="flex flex-col gap-6">
      {/* Page Header */}
      <div>
        <h2 className="text-xl font-semibold">General</h2>
        <p className="text-default-500 text-sm">
          Application-wide settings and preferences
        </p>
      </div>

      {/* Placeholder */}
      <Card>
        <CardBody className="py-16 text-center">
          <IconSettings size={48} className="mx-auto text-default-300 mb-4" />
          <p className="text-default-500">No general settings available yet</p>
          <p className="text-default-400 text-sm mt-1">
            Additional configuration options will appear here in future updates.
          </p>
        </CardBody>
      </Card>
    </div>
  )
}
