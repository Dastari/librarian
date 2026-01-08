import { createFileRoute, redirect } from '@tanstack/react-router'
import { VideoPlayer } from '../../components/VideoPlayer'
import type { MediaItem } from '../../lib/api'

export const Route = createFileRoute('/library/$mediaId')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/auth/login' })
    }
  },
  component: MediaDetailPage,
})

// Mock data
const mockMedia: MediaItem = {
  id: '1',
  title: 'Example Movie',
  media_type: 'movie',
  year: 2024,
  overview: 'This is a sample movie description. Once you add real media to your library, you\'ll see actual metadata, posters, and streaming options here.',
  runtime: 120,
  poster_url: null,
  backdrop_url: null,
}

function MediaDetailPage() {
  const { mediaId } = Route.useParams()

  // TODO: Fetch actual media data
  const media = mockMedia

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Backdrop */}
      <div className="relative rounded-xl overflow-hidden mb-8 bg-gradient-to-r from-slate-800 to-slate-900 aspect-video max-h-96">
        {media.backdrop_url ? (
          <img
            src={media.backdrop_url}
            alt={media.title}
            className="w-full h-full object-cover"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center">
            <VideoPlayer 
              src="" 
              poster={media.poster_url || undefined}
            />
          </div>
        )}
      </div>

      {/* Details */}
      <div className="grid grid-cols-1 md:grid-cols-[200px_1fr] gap-8">
        {/* Poster */}
        <div className="hidden md:block">
          <div className="aspect-[2/3] bg-slate-800 rounded-lg overflow-hidden">
            {media.poster_url ? (
              <img
                src={media.poster_url}
                alt={media.title}
                className="w-full h-full object-cover"
              />
            ) : (
              <div className="w-full h-full flex items-center justify-center">
                <span className="text-slate-500 text-6xl">
                  {media.media_type === 'movie' ? '🎬' : '📺'}
                </span>
              </div>
            )}
          </div>
        </div>

        {/* Info */}
        <div>
          <h1 className="text-3xl font-bold mb-2">{media.title}</h1>
          
          <div className="flex items-center gap-4 text-slate-400 mb-4">
            {media.year && <span>{media.year}</span>}
            {media.runtime && <span>{media.runtime} min</span>}
            <span className="capitalize">{media.media_type}</span>
          </div>

          <p className="text-slate-300 mb-6 max-w-2xl">
            {media.overview || 'No description available.'}
          </p>

          {/* Actions */}
          <div className="flex flex-wrap gap-4">
            <button className="bg-blue-600 hover:bg-blue-700 text-white font-semibold px-6 py-3 rounded-lg transition-colors flex items-center gap-2">
              <span>▶</span> Play
            </button>
            <button className="bg-slate-700 hover:bg-slate-600 text-white font-semibold px-6 py-3 rounded-lg transition-colors">
              Cast
            </button>
            <button className="bg-slate-700 hover:bg-slate-600 text-white font-semibold px-6 py-3 rounded-lg transition-colors">
              ⚙️ Options
            </button>
          </div>

          {/* Media info */}
          <div className="mt-8 bg-slate-800/50 rounded-lg p-4">
            <h3 className="font-semibold mb-3">Media Information</h3>
            <dl className="grid grid-cols-2 gap-2 text-sm">
              <dt className="text-slate-400">Media ID</dt>
              <dd>{mediaId}</dd>
              <dt className="text-slate-400">Type</dt>
              <dd className="capitalize">{media.media_type}</dd>
              <dt className="text-slate-400">Resolution</dt>
              <dd>1080p</dd>
              <dt className="text-slate-400">Video Codec</dt>
              <dd>H.264</dd>
              <dt className="text-slate-400">Audio Codec</dt>
              <dd>AAC</dd>
            </dl>
          </div>
        </div>
      </div>
    </div>
  )
}
