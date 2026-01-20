import { createFileRoute, Outlet, redirect } from '@tanstack/react-router'
import { Link, useLocation } from '@tanstack/react-router'
import { Card, CardBody } from '@heroui/card'
import { RouteError } from '../components/RouteError'
import type { TablerIcon } from '@tabler/icons-react'
import { IconSettings, IconDownload, IconRss, IconMovie, IconClipboard, IconSearch, IconCast, IconFolderCog, IconBrain } from '@tabler/icons-react'

// This is the parent route for /settings/* that provides the shared layout
export const Route = createFileRoute('/settings')({
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: '/',
        search: {
          signin: true,
          redirect: location.href,
        },
      })
    }
  },
  component: SettingsLayoutRoute,
  errorComponent: RouteError,
})

interface SettingsTab {
  key: string
  path: string
  label: string
  Icon: TablerIcon
  iconColor: string
  description: string
}

const settingsTabs: SettingsTab[] = [
  {
    key: 'general',
    path: '/settings',
    label: 'General',
    Icon: IconSettings,
    iconColor: 'text-default-400',
    description: 'App preferences',
  },
  {
    key: 'torrent',
    path: '/settings/torrent',
    label: 'Torrent Client',
    Icon: IconDownload,
    iconColor: 'text-blue-400',
    description: 'Download settings',
  },
  {
    key: 'indexers',
    path: '/settings/indexers',
    label: 'Indexers',
    Icon: IconSearch,
    iconColor: 'text-green-400',
    description: 'Search & encryption',
  },
  {
    key: 'rss',
    path: '/settings/rss',
    label: 'RSS Feeds',
    Icon: IconRss,
    iconColor: 'text-orange-400',
    description: 'Torrent feed sources',
  },
  {
    key: 'metadata',
    path: '/settings/metadata',
    label: 'Metadata',
    Icon: IconMovie,
    iconColor: 'text-purple-400',
    description: 'Media identification',
  },
  {
    key: 'parser',
    path: '/settings/parser',
    label: 'Filename Parser',
    Icon: IconBrain,
    iconColor: 'text-cyan-400',
    description: 'Regex & LLM parsing',
  },
  {
    key: 'organization',
    path: '/settings/organization',
    label: 'File Organization',
    Icon: IconFolderCog,
    iconColor: 'text-amber-400',
    description: 'Naming patterns',
  },
  {
    key: 'casting',
    path: '/settings/casting',
    label: 'Casting',
    Icon: IconCast,
    iconColor: 'text-teal-400',
    description: 'Chromecast devices',
  },
  {
    key: 'logs',
    path: '/settings/logs',
    label: 'System Logs',
    Icon: IconClipboard,
    iconColor: 'text-default-400',
    description: 'Activity & errors',
  },
]

function SettingsLayoutRoute() {
  const location = useLocation()

  const isActive = (path: string) => {
    if (path === '/settings') {
      return location.pathname === '/settings' || location.pathname === '/settings/'
    }
    return location.pathname.startsWith(path)
  }

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col h-full overflow-hidden">
      <h1 className="text-2xl font-bold mb-6 shrink-0">Settings</h1>

      <div className="flex flex-col lg:flex-row gap-6 flex-1 min-h-0 overflow-hidden">
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
                    <tab.Icon size={20} className={isActive(tab.path) ? '' : tab.iconColor} />
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

        {/* Right Content Area - scrolls independently */}
        <div className="flex-1 min-h-0 overflow-y-auto px-4 -mx-4">
          <div className="flex flex-col h-full">
            <Outlet />
          </div>
        </div>
      </div>
    </div>
  )
}
