import { supabase } from './supabase'

const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001'

async function getAuthHeaders(): Promise<HeadersInit> {
  const { data: { session } } = await supabase.auth.getSession()
  
  return {
    'Content-Type': 'application/json',
    ...(session?.access_token && { 'Authorization': `Bearer ${session.access_token}` }),
  }
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const headers = await getAuthHeaders()
  
  const response = await fetch(`${API_URL}${path}`, {
    ...options,
    headers: {
      ...headers,
      ...options.headers,
    },
  })
  
  if (!response.ok) {
    throw new Error(`API error: ${response.status}`)
  }
  
  return response.json()
}

// Health check
export const checkHealth = () => request<{ status: string; version: string }>('/healthz')

// User
export const getMe = () => request<{ id: string; email: string | null }>('/api/me')

// Libraries
export type LibraryType = 'movies' | 'tv' | 'music' | 'audiobooks' | 'other'

export interface Library {
  id: string
  name: string
  path: string
  library_type: LibraryType
  icon: string
  color: string
  auto_scan: boolean
  scan_interval_hours: number
  last_scanned_at: string | null
  file_count: number | null
  total_size_bytes: number | null
}

export interface CreateLibraryRequest {
  name: string
  path: string
  library_type: LibraryType
  icon?: string
  color?: string
  auto_scan?: boolean
  scan_interval_hours?: number
}

export interface LibraryStats {
  library_id: string
  file_count: number
  total_size_bytes: number
  movie_count: number
  episode_count: number
  last_scanned_at: string | null
}

export const getLibraries = () => request<Library[]>('/api/libraries')

export const getLibrary = (id: string) => request<Library>(`/api/libraries/${id}`)

export const createLibrary = (data: CreateLibraryRequest) => 
  request<Library>('/api/libraries', {
    method: 'POST',
    body: JSON.stringify(data),
  })

export const updateLibrary = (id: string, data: Partial<CreateLibraryRequest>) =>
  request<Library>(`/api/libraries/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(data),
  })

export const deleteLibrary = (id: string) =>
  request<void>(`/api/libraries/${id}`, { method: 'DELETE' })

export const scanLibrary = (id: string) => 
  request<{ status: string; library_id: string }>(`/api/libraries/${id}/scan`, { method: 'POST' })

export const getLibraryStats = (id: string) =>
  request<LibraryStats>(`/api/libraries/${id}/stats`)

// Library type helpers
export const LIBRARY_TYPES: { value: LibraryType; label: string; icon: string; color: string }[] = [
  { value: 'movies', label: 'Movies', icon: '🎬', color: 'purple' },
  { value: 'tv', label: 'TV Shows', icon: '📺', color: 'blue' },
  { value: 'music', label: 'Music', icon: '🎵', color: 'green' },
  { value: 'audiobooks', label: 'Audiobooks', icon: '🎧', color: 'orange' },
  { value: 'other', label: 'Other', icon: '📁', color: 'slate' },
]

export const getLibraryTypeInfo = (type: LibraryType) =>
  LIBRARY_TYPES.find(t => t.value === type) || LIBRARY_TYPES[4]

// Media
export interface MediaItem {
  id: string
  title: string
  media_type: 'movie' | 'episode'
  year: number | null
  overview: string | null
  runtime: number | null
  poster_url: string | null
  backdrop_url: string | null
}

export const getMedia = (id: string) => request<MediaItem | null>(`/api/media/${id}`)

export const getStreamUrl = (id: string) => 
  request<{ playlist_url: string; direct_play_supported: boolean }>(`/api/media/${id}/stream/hls`)

// Torrents
export interface Torrent {
  hash: string
  name: string
  state: string
  progress: number
  size: number
  download_speed: number
  upload_speed: number
}

export const getTorrents = () => request<Torrent[]>('/api/torrents')

export const addTorrent = (data: { url?: string; magnet?: string; category?: string }) =>
  request<{ status: string }>('/api/torrents', {
    method: 'POST',
    body: JSON.stringify(data),
  })

// Subscriptions
export interface Subscription {
  id: string
  show_name: string
  tvdb_id: number
  quality_profile_id: string
  monitored: boolean
}

export const getSubscriptions = () => request<Subscription[]>('/api/subscriptions')

export const createSubscription = (data: Omit<Subscription, 'id'>) =>
  request<Subscription>('/api/subscriptions', {
    method: 'POST',
    body: JSON.stringify(data),
  })
