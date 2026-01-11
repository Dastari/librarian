import { Link, useLocation } from '@tanstack/react-router'
import { Card, CardBody } from '@heroui/card'
import { Spinner } from '@heroui/spinner'
import { ScrollShadow } from '@heroui/scroll-shadow'
import type { ReactNode } from 'react'

interface SettingsTab {
  key: string
  path: string
  label: string
  icon: string
  description: string
}

const settingsTabs: SettingsTab[] = [
  {
    key: 'torrent',
    path: '/settings',
    label: 'Torrent Client',
    icon: '⬇️',
    description: 'Download settings',
  },
  {
    key: 'rss',
    path: '/settings/rss',
    label: 'RSS Feeds',
    icon: '📡',
    description: 'Torrent feed sources',
  },
  {
    key: 'metadata',
    path: '/settings/metadata',
    label: 'Metadata & Parser',
    icon: '🎬',
    description: 'Media identification',
  },
  {
    key: 'logs',
    path: '/settings/logs',
    label: 'System Logs',
    icon: '📋',
    description: 'Activity & errors',
  },
]

interface SettingsLayoutProps {
  children: ReactNode
  isLoading?: boolean
}

export function SettingsLayout({ children, isLoading }: SettingsLayoutProps) {
  const location = useLocation()

  const isActive = (path: string) => {
    if (path === '/settings') {
      return location.pathname === '/settings' || location.pathname === '/settings/'
    }
    return location.pathname.startsWith(path)
  }

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col grow">
      <h1 className="text-2xl font-bold mb-6 flex-shrink-0">Settings</h1>

      <div className="flex flex-col lg:flex-row gap-6 flex-1 min-h-0">
        {/* Left Sidebar - Vertical Tabs */}
        <div className="lg:w-64 flex-shrink-0">
          <Card className="sticky top-4">
            <CardBody className="p-2">
              <nav className="flex flex-col gap-1">
                {settingsTabs.map((tab) => (
                  <Link
                    key={tab.key}
                    to={tab.path}
                    className={`
                      flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200
                      ${isActive(tab.path)
                        ? 'bg-primary text-primary-foreground shadow-md'
                        : 'hover:bg-content2 text-default-600 hover:text-foreground'
                      }
                    `}
                  >
                    <span className="text-xl">{tab.icon}</span>
                    <div className="flex flex-col">
                      <span className="font-medium text-sm">{tab.label}</span>
                      <span
                        className={`text-xs ${isActive(tab.path)
                          ? 'text-primary-foreground/70'
                          : 'text-default-400'
                          }`}
                      >
                        {tab.description}
                      </span>
                    </div>
                  </Link>
                ))}
              </nav>
            </CardBody>
          </Card>
        </div>

        {/* Right Content Area */}
        <ScrollShadow className="flex-1 min-w-0">
          {isLoading ? (
            <div className="flex justify-center items-center py-20">
              <Spinner size="lg" />
            </div>
          ) : (
            <div className="pb-4 flex h-full grow h-0">{children}</div>
          )}
        </ScrollShadow>
      </div>
    </div>
  )
}
