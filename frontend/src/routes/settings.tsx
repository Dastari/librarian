import { createFileRoute, Outlet, redirect } from '@tanstack/react-router'
import { Link, useLocation } from '@tanstack/react-router'
import { Card, CardBody } from '@heroui/card'
import { RouteError } from '../components/RouteError'
import { isAdmin } from '../lib/auth'
import type { TablerIcon } from '@tabler/icons-react'
import { IconArchive, IconSettings, IconDownload, IconMovie, IconClipboard, IconCast, IconFolderCog, IconRadar, IconAdjustmentsHorizontal } from '@tabler/icons-react'

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

    if (!isAdmin(context.auth.user)) {
      throw redirect({ to: '/libraries' })
    }
  },
  component: SettingsLayoutRoute,
  errorComponent: RouteError,
})

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
    key: 'general',
    path: '/settings',
    label: 'General',
    icon: IconSettings,
    iconColor: 'text-default-400',
    description: 'App preferences',
  },
  {
    key: 'torrent',
    path: '/settings/torrent',
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
    label: 'Metadata',
    icon: IconMovie,
    iconColor: 'text-purple-400',
    description: 'Media identification',
  },
  {
    key: 'organization',
    path: '/settings/organization',
    label: 'File Organization',
    icon: IconFolderCog,
    iconColor: 'text-amber-400',
    description: 'Naming patterns',
  },
  {
    key: 'quality-profiles',
    path: '/settings/quality-profiles',
    label: 'Quality Profiles',
    icon: IconAdjustmentsHorizontal,
    iconColor: 'text-pink-400',
    description: 'Resolution, codec & HDR rules',
  },
  {
    key: 'casting',
    path: '/settings/casting',
    label: 'Casting',
    icon: IconCast,
    iconColor: 'text-teal-400',
    description: 'Chromecast devices',
  },
  { key: 'backup', path: '/settings/backup', label: 'Backup', icon: IconArchive, iconColor: 'text-blue-400', description: 'Snapshots & verification' },
  {
    key: 'logs',
    path: '/settings/logs',
    label: 'System Logs',
    icon: IconClipboard,
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
              <nav aria-label="Settings sections" className="flex flex-row lg:flex-col gap-1 overflow-x-auto">
                {settingsTabs.map((tab) => (
                  <Link
                    key={tab.key}
                    to={tab.path}
                    className={`
                      flex shrink-0 items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200
                      ${isActive(tab.path)
                        ? 'bg-primary text-primary-foreground shadow-md'
                        : 'hover:bg-content2 text-default-600 hover:text-foreground'
                      }
                    `}
                  >
                    <tab.icon size={20} className={isActive(tab.path) ? '' : tab.iconColor} />
                    <div className="flex flex-col">
                      <span className="font-medium text-sm whitespace-nowrap">{tab.label}</span>
                      <span
                        className={`hidden lg:block text-xs ${isActive(tab.path)
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
