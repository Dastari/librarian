/**
 * Animated Playing Indicator
 * 
 * Shows animated equalizer bars when audio/video is playing.
 * Used to indicate currently playing track/episode/chapter in lists.
 */

import { memo } from 'react';

interface PlayingIndicatorProps {
  /** Whether the animation should play (true when playing, false when paused) */
  isAnimating?: boolean;
  /** Size of the indicator in pixels */
  size?: number;
  /** Color class for the bars */
  colorClass?: string;
}

export const PlayingIndicator = memo(function PlayingIndicator({
  isAnimating = true,
  size = 16,
  colorClass = 'bg-primary',
}: PlayingIndicatorProps) {
  const barWidth = Math.max(2, size / 5);
  const gap = Math.max(1, size / 8);

  return (
    <div
      className="flex items-end justify-center"
      style={{ width: size, height: size, gap }}
    >
      {[0, 1, 2, 3].map((i) => (
        <div
          key={i}
          className={`${colorClass} rounded-sm ${isAnimating ? 'animate-equalizer' : ''}`}
          style={{
            width: barWidth,
            height: isAnimating ? undefined : size * 0.3,
            minHeight: size * 0.2,
            animationDelay: isAnimating ? `${i * 0.1}s` : undefined,
          }}
        />
      ))}

      {/* CSS animation defined inline for the equalizer effect */}
      <style>{`
        @keyframes equalizer {
          0%, 100% { height: ${size * 0.2}px; }
          25% { height: ${size * 0.8}px; }
          50% { height: ${size * 0.4}px; }
          75% { height: ${size * 1}px; }
        }
        .animate-equalizer {
          animation: equalizer 0.8s ease-in-out infinite;
        }
      `}</style>
    </div>
  );
});
