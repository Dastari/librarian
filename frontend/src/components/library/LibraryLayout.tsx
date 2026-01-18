import type { ReactNode } from 'react'
import { Link } from '@tanstack/react-router'
import { Card, CardBody } from '@heroui/card'
import { Button } from '@heroui/button'
import type { TablerIcon } from '@tabler/icons-react'
import { IconDeviceTv, IconFileSearch, IconFolder, IconSettings } from '@tabler/icons-react'

export type LibraryTab = 'shows' | 'unmatched' | 'browser' | 'settings'

interface LibraryTabConfig {
  key: LibraryTab
  label: string
  Icon: TablerIcon
  iconColor: string
  description: string
  position?: 'top' | 'bottom'
  path: string // relative path for the tab
}

const libraryTabs: LibraryTabConfig[] = [
  {
    key: 'shows',
    label: 'Shows',
    Icon: IconDeviceTv,
    iconColor: 'text-blue-400',
    description: 'TV shows in library',
    position: 'top',
    path: 'shows',
  },
  {
    key: 'unmatched',
    label: 'Unmatched Files',
    Icon: IconFileSearch,
    iconColor: 'text-amber-400',
    description: 'Files without matches',
    position: 'top',
    path: 'unmatched',
  },
  {
    key: 'browser',
    label: 'File Browser',
    Icon: IconFolder,
    iconColor: 'text-amber-400',
    description: 'Browse library files',
    position: 'top',
    path: 'browser',
  },
  {
    key: 'settings',
    label: 'Settings',
    Icon: IconSettings,
    iconColor: 'text-default-400',
    description: 'Library configuration',
    position: 'bottom',
    path: 'settings',
  },
]

interface LibraryLayoutProps {
  activeTab: LibraryTab
  libraryId: string
  children: ReactNode
}

export function LibraryLayout({ activeTab, libraryId, children }: LibraryLayoutProps) {
  const topTabs = libraryTabs.filter((tab) => tab.position !== 'bottom')
  const bottomTabs = libraryTabs.filter((tab) => tab.position === 'bottom')

  const renderTabButton = (tab: LibraryTabConfig) => {
    const isActive = activeTab === tab.key
    return (
      <Button
        key={tab.key}
        as={Link}
        to={`/libraries/${libraryId}/${tab.path}`}
        variant={isActive ? 'solid' : 'light'}
        color={isActive ? 'primary' : 'default'}
        className={`
          flex items-center gap-3 px-4 py-3 h-auto justify-start text-left w-full
          ${isActive ? 'shadow-md' : ''}
        `}
      >
        <tab.Icon className={`w-5 h-5 ${isActive ? '' : tab.iconColor}`} />
        <div className="flex flex-col min-w-0 items-start">
          <span className="font-medium text-sm">{tab.label}</span>
          <span
            className={`text-xs truncate ${isActive
                ? 'text-primary-foreground/70'
                : 'text-default-400'
              }`}
          >
            {tab.description}
          </span>
        </div>
      </Button>
    )
  }

  return (
    <div className="flex gap-6 h-[calc(100vh-15rem)] min-h-[500px]">
      {/* Left Sidebar - Fixed Vertical Tabs */}
      <div className="w-56 shrink-0">
        <Card className="h-full">
          <CardBody className="p-2 flex flex-col">
            <nav className="flex flex-col gap-1 flex-1">
              {topTabs.map(renderTabButton)}
            </nav>

            {/* Bottom tabs (Settings) */}
            <div className="border-t border-divider pt-2 mt-2">
              {bottomTabs.map(renderTabButton)}
            </div>
          </CardBody>
        </Card>
      </div>

      {/* Right Content Area */}
      <div className="flex grow flex-col w-full">
        <div className="flex grow h-0 overflow-auto">
          {children}
        </div>
      </div>
    </div>
  )
}
