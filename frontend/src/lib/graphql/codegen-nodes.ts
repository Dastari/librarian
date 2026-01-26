/**
 * Re-export codegen node types for use across the app.
 * Use these types and PascalCase field names (.Id, .Name, .LibraryType, etc.)
 * instead of converting to legacy camelCase types.
 */

import type {
  LibrariesQuery,
  DashboardShowsQuery,
  DashboardScheduleCachesQuery,
} from './generated/graphql'

/** Library node from Libraries query (PascalCase) */
export type LibraryNode = LibrariesQuery['Libraries']['Edges'][0]['Node']

/** Show node from Shows query (PascalCase) */
export type ShowNode = DashboardShowsQuery['Shows']['Edges'][0]['Node']

/** ScheduleCache node from ScheduleCaches query (PascalCase) */
export type ScheduleCacheNode =
  DashboardScheduleCachesQuery['ScheduleCaches']['Edges'][0]['Node']
