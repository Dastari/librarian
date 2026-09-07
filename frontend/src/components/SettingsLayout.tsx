import { Link, useLocation } from '@tanstack/react-router'
import { Card, CardBody } from '@heroui/card'
import { Spinner } from '@heroui/spinner'
import { ScrollShadow } from '@heroui/scroll-shadow'
import type { ReactNode } from 'react'
import type { TablerIcon } from '@tabler/icons-react'
import { IconDownload, IconMovie, IconClipboard, IconCast, IconRadar, IconArchive } from '@tabler/icons-react'

interface SettingsTab {
  key: string
  path: string
  label: string
  icon: TablerIcon
  iconColor: string
  description: string
}

const settingsTabs: SettingsTab[] = [
  {
    key: 'torrent',
    path: '/settings',
    label: 'Torrent Client',
    icon: IconDownload,
    iconColor: 'text-blue-400',
    description: 'Download settings',
  },
  {
    key: 'sources',
    path: '/settings/sources',
    label: 'Sources',
    icon: IconRadar,
    iconColor: 'text-green-400',
    description: 'Torrent indexers, RSS feeds & source ordering',
  },
  {
    key: 'metadata',
    path: '/settings/metadata',
    label: 'Metadata & Parser',
    icon: IconMovie,
    iconColor: 'text-purple-400',
    description: 'Media identification',
  },
  {
    key: 'casting',
    path: '/settings/casting',
    label: 'Casting',
    icon: IconCast,
    iconColor: 'text-teal-400',
    description: 'Chromecast devices',
  },
  {
    key: 'logs',
    path: '/settings/logs',
    label: 'System Logs',
    icon: IconClipboard,
    iconColor: 'text-default-400',
    description: 'Activity & errors',
  },
  {
    key: 'backup',
    path: '/settings/backup',
    label: 'Backup',
    icon: IconArchive,
    iconColor: 'text-amber-400',
    description: 'Storage snapshots',
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
    <div className="container mx-auto flex h-full min-h-0 grow flex-col overflow-hidden px-4 py-8 sm:px-6 lg:px-8">
      <h1 className="text-2xl font-bold mb-6 shrink-0">Settings</h1>

      <div className="flex min-h-0 flex-1 flex-col gap-6 overflow-hidden lg:flex-row">
        {/* Left Sidebar - Vertical Tabs */}
        <div className="lg:w-64 shrink-0">
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
                    <tab.icon className={`w-5 h-5 ${isActive(tab.path) ? '' : tab.iconColor}`} />
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
        <ScrollShadow className="-mx-4 h-0 min-h-0 min-w-0 flex-1 px-4">
          {isLoading ? (
            <div className="flex justify-center items-center py-20">
              <Spinner size="lg" />
            </div>
          ) : (
            <div className="pb-4 flex flex-col h-full grow">{children}</div>
          )}
        </ScrollShadow>
      </div>
    </div>
  )
}
