import React, { useState, useRef, useLayoutEffect, useMemo, useEffect, Children, cloneElement, isValidElement } from 'react'

interface ShimmerRect {
  x: number
  y: number
  width: number
  height: number
  borderRadius: string
}

interface ShimmerLoaderProps {
  loading: boolean
  templateProps?: Record<string, unknown>
  children: React.ReactNode
  /** Color of the shimmer wave */
  shimmerColor?: string
  /** Background color of shimmer blocks */
  backgroundColor?: string
  /** Animation duration in seconds */
  duration?: number
  /** Fallback border radius for elements with no CSS border-radius */
  fallbackBorderRadius?: number
  /** Delay in ms before showing shimmer (avoids flash for fast loads) */
  delay?: number
}

/**
 * Collects all leaf elements (elements with no children or only text) from the DOM tree
 */
function collectLeafElements(element: Element, containerRect: DOMRect): ShimmerRect[] {
  const rects: ShimmerRect[] = []

  function traverse(el: Element) {
    const children = Array.from(el.children)
    
    // If no child elements, this is a leaf node
    if (children.length === 0) {
      const rect = el.getBoundingClientRect()
      // Only include visible elements with dimensions
      if (rect.width > 0 && rect.height > 0) {
        const computedStyle = window.getComputedStyle(el)
        rects.push({
          x: rect.left - containerRect.left,
          y: rect.top - containerRect.top,
          width: rect.width,
          height: rect.height,
          borderRadius: computedStyle.borderRadius || '0px',
        })
      }
    } else {
      // Traverse children
      children.forEach(child => traverse(child))
    }
  }

  traverse(element)
  return rects
}

/**
 * Auto-generates shimmer skeletons from actual component structure.
 * Renders the component with transparent text, measures the DOM,
 * and generates shimmer overlays that match exact dimensions.
 *
 * Usage:
 * ```tsx
 * <ShimmerLoader loading={isLoading} templateProps={{ data: templateData }}>
 *   <MyComponent data={realData ?? templateData} />
 * </ShimmerLoader>
 * ```
 */
export function ShimmerLoader({
  loading,
  templateProps,
  children,
  shimmerColor = 'rgba(255, 255, 255, 0.08)',
  backgroundColor = 'rgba(255, 255, 255, 0.04)',
  duration = 1.5,
  fallbackBorderRadius = 8,
  delay = 0,
}: ShimmerLoaderProps) {
  const [rects, setRects] = useState<ShimmerRect[]>([])
  const [showShimmer, setShowShimmer] = useState(delay === 0)
  const measureRef = useRef<HTMLDivElement>(null)
  const overlayRef = useRef<HTMLDivElement>(null)

  // Handle delayed shimmer display
  useEffect(() => {
    if (!loading) {
      // Reset when loading stops
      setShowShimmer(delay === 0)
      return
    }

    if (delay === 0) {
      setShowShimmer(true)
      return
    }

    // Start timer to show shimmer after delay
    const timer = setTimeout(() => {
      setShowShimmer(true)
    }, delay)

    return () => clearTimeout(timer)
  }, [loading, delay])

  // Inject templateProps into the first child when loading
  const childrenWithProps = useMemo(() => {
    if (!loading || !templateProps) {
      return children
    }

    const childArray = Children.toArray(children)
    if (childArray.length === 0) {
      return children
    }

    const firstChild = childArray[0]
    if (isValidElement(firstChild)) {
      return [
        cloneElement(firstChild, { ...templateProps }),
        ...childArray.slice(1),
      ]
    }
    return children
  }, [children, loading, templateProps])

  // Measure DOM structure when loading
  useLayoutEffect(() => {
    if (!loading || !measureRef.current) {
      return
    }

    const container = measureRef.current
    const containerRect = container.getBoundingClientRect()
    const measuredRects: ShimmerRect[] = []

    Array.from(container.children).forEach(child => {
      measuredRects.push(...collectLeafElements(child, containerRect))
    })

    setRects(measuredRects)
  }, [loading, childrenWithProps])

  if (!loading) {
    return <>{children}</>
  }

  // If loading but delay hasn't passed, render nothing (no flash)
  if (!showShimmer) {
    return null
  }

  return (
    <div style={{ position: 'relative' }}>
      {/* CSS for hiding text/images during measurement */}
      <style>{`
        .shimmer-measure-container * {
          color: transparent !important;
          border-color: transparent !important;
        }
        .shimmer-measure-container img,
        .shimmer-measure-container svg,
        .shimmer-measure-container video {
          opacity: 0;
        }
        @keyframes shimmer-slide {
          0% {
            transform: translateX(-100%);
          }
          100% {
            transform: translateX(100%);
          }
        }
      `}</style>

      {/* Hidden container for DOM measurement */}
      <div
        ref={measureRef}
        className="shimmer-measure-container"
        style={{ pointerEvents: 'none' }}
        aria-hidden="true"
      >
        {childrenWithProps}
      </div>

      {/* Shimmer overlay */}
      <div
        ref={overlayRef}
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          overflow: 'hidden',
        }}
      >
        {rects.map((rect, index) => (
          <div
            key={index}
            style={{
              position: 'absolute',
              left: `${rect.x}px`,
              top: `${rect.y}px`,
              width: `${rect.width}px`,
              height: `${rect.height}px`,
              backgroundColor,
              borderRadius: rect.borderRadius === '0px' ? `${fallbackBorderRadius}px` : rect.borderRadius,
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: '100%',
                background: `linear-gradient(90deg, transparent, ${shimmerColor}, transparent)`,
                animation: `shimmer-slide ${duration}s infinite`,
              }}
            />
          </div>
        ))}
      </div>
    </div>
  )
}
