import type { ReactNode } from 'react'
import { Link } from '@tanstack/react-router'
import { Card, CardBody } from '@heroui/card'
import { Button } from '@heroui/button'
import type { TablerIcon } from '@tabler/icons-react'
import {
  IconDeviceTv,
  IconMovie,
  IconHeadphones,
  IconFileSearch,
  IconFolder,
  IconSettings,
  IconMicrophone,
  IconDisc,
  IconMusicBolt,
  IconUser,
  IconStack,
} from '@tabler/icons-react'
import type { LibraryType } from '../../lib/graphql'

// Extended tab keys for all library types
export type LibraryTab =
  // TV
  | 'shows'
  // Movies
  | 'movies'
  | 'collections'
  // Music
  | 'artists'
  | 'albums'
  | 'tracks'
  // Audiobooks
  | 'books'
  | 'authors'
  // Common
  | 'unmatched'
  | 'browser'
  | 'settings'

interface LibraryTabConfig {
  key: LibraryTab
  label: string
  Icon: TablerIcon
  iconColor: string
  description: string
  position?: 'top' | 'bottom'
  path: string // relative path for the tab
}

// TV Show tabs
const tvTabs: LibraryTabConfig[] = [
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

// Movie tabs
const movieTabs: LibraryTabConfig[] = [
  {
    key: 'movies',
    label: 'Movies',
    Icon: IconMovie,
    iconColor: 'text-purple-400',
    description: 'Movies in library',
    position: 'top',
    path: 'movies',
  },
  {
    key: 'collections',
    label: 'Collections',
    Icon: IconStack,
    iconColor: 'text-purple-300',
    description: 'Movie collections',
    position: 'top',
    path: 'collections',
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

// Music tabs
const musicTabs: LibraryTabConfig[] = [
  {
    key: 'artists',
    label: 'Artists',
    Icon: IconMicrophone,
    iconColor: 'text-green-400',
    description: 'Artists in library',
    position: 'top',
    path: 'artists',
  },
  {
    key: 'albums',
    label: 'Albums',
    Icon: IconDisc,
    iconColor: 'text-green-300',
    description: 'Albums in library',
    position: 'top',
    path: 'albums',
  },
  {
    key: 'tracks',
    label: 'Tracks',
    Icon: IconMusicBolt,
    iconColor: 'text-green-200',
    description: 'All tracks',
    position: 'top',
    path: 'tracks',
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

// Audiobook tabs
const audiobookTabs: LibraryTabConfig[] = [
  {
    key: 'books',
    label: 'Audiobooks',
    Icon: IconHeadphones,
    iconColor: 'text-orange-400',
    description: 'Audiobooks in library',
    position: 'top',
    path: 'books',
  },
  {
    key: 'authors',
    label: 'Authors',
    Icon: IconUser,
    iconColor: 'text-orange-300',
    description: 'Authors in library',
    position: 'top',
    path: 'authors',
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

// Other/generic tabs
const otherTabs: LibraryTabConfig[] = [
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

/**
 * Get tabs for a specific library type
 */
export function getTabsForLibraryType(libraryType: LibraryType): LibraryTabConfig[] {
  switch (libraryType) {
    case 'MOVIES':
      return movieTabs
    case 'TV':
      return tvTabs
    case 'MUSIC':
      return musicTabs
    case 'AUDIOBOOKS':
      return audiobookTabs
    case 'OTHER':
    default:
      return otherTabs
  }
}

/**
 * Get the default tab for a library type
 */
export function getDefaultTabForLibraryType(libraryType: LibraryType): LibraryTab {
  switch (libraryType) {
    case 'MOVIES':
      return 'movies'
    case 'TV':
      return 'shows'
    case 'MUSIC':
      return 'albums'
    case 'AUDIOBOOKS':
      return 'books'
    case 'OTHER':
    default:
      return 'browser'
  }
}

interface LibraryLayoutProps {
  activeTab: LibraryTab
  libraryId: string
  libraryType: LibraryType
  children: ReactNode
}

export function LibraryLayout({ activeTab, libraryId, libraryType, children }: LibraryLayoutProps) {
  const tabs = getTabsForLibraryType(libraryType)
  const topTabs = tabs.filter((tab) => tab.position !== 'bottom')
  const bottomTabs = tabs.filter((tab) => tab.position === 'bottom')

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
            {bottomTabs.length > 0 && (
              <div className="pt-2 mt-2">
                {bottomTabs.map(renderTabButton)}
              </div>
            )}
          </CardBody>
        </Card>
      </div>

      {/* Right Content Area */}
      <div className="flex grow flex-col w-full">
        <div className="flex grow h-0 overflow-auto px-4 -mx-4">
          {children}
        </div>
      </div>
    </div>
  )
}
