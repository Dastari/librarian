import { useMemo } from 'react'

// Sample album art - these would come from your actual media library
const sampleCovers = [
  'https://images.unsplash.com/photo-1611348586804-61bf6c080437?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1614613535308-eb5fbd3d2c17?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1470225620780-dba8ba36b745?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1493225457124-a3eb161ffa5f?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1459749411175-04bf5292ceea?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1514525253161-7a46d19cd819?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1506157786151-b8491531f063?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1460667262436-cf19894f4774?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1619983081563-430f63602796?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1571330735066-03aaa9429d89?w=200&h=200&fit=crop',
  'https://images.unsplash.com/photo-1504509546545-e000b4a62425?w=200&h=200&fit=crop',
]

interface CoverPosition {
  id: number
  x: number
  z: number
  speed: number
  size: number
  src: string
  yOffset: number
}

export function AlbumArtCarousel() {
  const covers = useMemo(() => {
    // Generate random positions for covers across 3 depth layers
    const generated: CoverPosition[] = []
    const layers = [
      { z: -150, count: 6, speed: 45, size: 90, opacity: 0.4 },   // Far back - slow, small
      { z: -50, count: 5, speed: 32, size: 110, opacity: 0.6 },   // Back layer
      { z: 50, count: 4, speed: 22, size: 130, opacity: 0.8 },    // Middle layer
      { z: 150, count: 3, speed: 16, size: 150, opacity: 1 },     // Front layer - fast, large
    ]

    let id = 0
    layers.forEach((layer, _layerIdx) => {
      for (let i = 0; i < layer.count; i++) {
        generated.push({
          id: id,
          x: (i / layer.count) * 100 + Math.random() * 15,
          z: layer.z + Math.random() * 40 - 20,
          speed: layer.speed + Math.random() * 8 - 4,
          size: layer.size + Math.random() * 20 - 10,
          src: sampleCovers[id % sampleCovers.length],
          yOffset: Math.random() * 30 - 15, // Vertical variance
        })
        id++
      }
    })

    return generated
  }, [])

  return (
    <div className="absolute inset-0 overflow-hidden">
      {/* Perspective container - positioned to create floor illusion */}
      <div
        className="absolute inset-0"
        style={{
          perspective: '800px',
          perspectiveOrigin: '50% 65%',
          transformStyle: 'preserve-3d',
        }}
      >
        {/* 3D Stage - rotated to create floor perspective */}
        <div
          className="absolute w-full"
          style={{
            top: '15%',
            height: '70%',
            transform: 'rotateX(25deg)',
            transformStyle: 'preserve-3d',
            transformOrigin: 'center bottom',
          }}
        >
          {/* Album covers */}
          {covers.map((cover) => {
            // Calculate opacity based on z-depth (further = more faded)
            const depthOpacity = Math.max(0.3, Math.min(1, (cover.z + 200) / 350))
            
            return (
              <div
                key={cover.id}
                className="absolute"
                style={{
                  left: 0,
                  top: `calc(50% + ${cover.yOffset}px)`,
                  width: cover.size,
                  height: cover.size,
                  transform: `translateZ(${cover.z}px) translateY(-50%)`,
                  transformStyle: 'preserve-3d',
                  animation: `scroll-x-${cover.id} ${cover.speed}s linear infinite`,
                  animationDelay: `-${(cover.x / 100) * cover.speed}s`,
                  opacity: depthOpacity,
                }}
              >
                {/* Main cover */}
                <div
                  className="w-full h-full rounded-lg overflow-hidden"
                  style={{
                    boxShadow: `
                      0 8px 32px rgba(0, 0, 0, 0.4),
                      0 0 60px rgba(147, 51, 234, ${0.1 * depthOpacity}),
                      inset 0 1px 0 rgba(255,255,255,0.1)
                    `,
                    transform: 'rotateX(-25deg)', // Counter-rotate to face camera
                    transformOrigin: 'center bottom',
                  }}
                >
                  <img
                    src={cover.src}
                    alt=""
                    className="w-full h-full object-cover"
                    loading="lazy"
                  />
                </div>

                {/* Reflection on the "floor" */}
                <div
                  className="absolute w-full pointer-events-none"
                  style={{
                    height: cover.size * 0.6,
                    top: '100%',
                    transform: 'rotateX(180deg) rotateX(-25deg) translateY(-4px)',
                    transformOrigin: 'center top',
                    overflow: 'hidden',
                    borderRadius: '0.5rem',
                  }}
                >
                  <div
                    style={{
                      width: '100%',
                      height: cover.size,
                      maskImage: 'linear-gradient(to bottom, rgba(0,0,0,0.35) 0%, transparent 80%)',
                      WebkitMaskImage: 'linear-gradient(to bottom, rgba(0,0,0,0.35) 0%, transparent 80%)',
                    }}
                  >
                    <img
                      src={cover.src}
                      alt=""
                      className="w-full h-full object-cover"
                      style={{ filter: 'blur(1px)' }}
                      loading="lazy"
                    />
                  </div>
                </div>
              </div>
            )
          })}
        </div>

        {/* Reflective floor surface */}
        <div
          className="absolute inset-x-0 bottom-0 h-1/2"
          style={{
            background: `
              linear-gradient(to bottom, 
                transparent 0%, 
                rgba(147, 51, 234, 0.02) 30%,
                rgba(59, 130, 246, 0.04) 60%,
                rgba(0, 0, 0, 0.3) 100%
              )
            `,
            transform: 'rotateX(60deg)',
            transformOrigin: 'bottom center',
          }}
        />
      </div>

      {/* Atmospheric overlays */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          background: `
            radial-gradient(ellipse 100% 60% at 50% 40%, transparent 0%, rgba(15, 23, 42, 0.7) 100%)
          `,
        }}
      />

      {/* Vignette */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          boxShadow: 'inset 0 0 150px 50px rgba(0, 0, 0, 0.5)',
        }}
      />

      {/* Bottom glow for reflection surface */}
      <div
        className="absolute inset-x-0 bottom-0 h-24 pointer-events-none"
        style={{
          background: 'linear-gradient(to top, rgba(147, 51, 234, 0.08), transparent)',
        }}
      />

      {/* Dynamic keyframe animations for each cover */}
      <style>{`
        ${covers.map((cover) => `
          @keyframes scroll-x-${cover.id} {
            from {
              left: calc(100% + ${cover.size}px);
            }
            to {
              left: -${cover.size + 50}px;
            }
          }
        `).join('\n')}

        @media (prefers-reduced-motion: reduce) {
          * {
            animation-duration: 0.01ms !important;
            animation-iteration-count: 1 !important;
          }
        }
      `}</style>
    </div>
  )
}
