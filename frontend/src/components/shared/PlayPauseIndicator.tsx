/**
 * Play/Pause Indicator with Hover Effect
 * 
 * Shows animated equalizer bars by default, but swaps to a pause icon on hover.
 * Used for the currently playing track/episode/chapter in lists.
 */

import { useState, memo } from 'react';
import { IconPlayerPause } from '@tabler/icons-react';
import { PlayingIndicator } from './PlayingIndicator';

interface PlayPauseIndicatorProps {
  /** Whether the media is currently playing */
  isPlaying: boolean;
  /** Size of the indicator in pixels */
  size?: number;
  /** Color class for the equalizer bars */
  colorClass?: string;
  /** Callback when clicked (to pause) */
  onPause?: () => void;
}

export const PlayPauseIndicator = memo(function PlayPauseIndicator({
  isPlaying,
  size = 16,
  colorClass = 'bg-success',
  onPause,
}: PlayPauseIndicatorProps) {
  const [isHovered, setIsHovered] = useState(false);

  // Only handle clicks if onPause is provided
  // Otherwise, let clicks bubble up to parent (e.g., DataTable's Button)
  const handleClick = onPause
    ? (e: React.MouseEvent) => {
        e.stopPropagation();
        onPause();
      }
    : undefined;

  return (
    <div
      className="relative cursor-pointer"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={handleClick}
      style={{ width: size, height: size }}
    >
      {/* Equalizer - shown when not hovered and playing */}
      <div
        className={`absolute inset-0 flex items-center justify-center transition-opacity duration-150 ${
          isHovered ? 'opacity-0' : 'opacity-100'
        }`}
      >
        <PlayingIndicator size={size} isAnimating={isPlaying} colorClass={colorClass} />
      </div>

      {/* Pause icon - shown on hover */}
      <div
        className={`absolute inset-0 flex items-center justify-center transition-opacity duration-150 ${
          isHovered ? 'opacity-100' : 'opacity-0'
        }`}
      >
        <IconPlayerPause size={size} className="text-warning" />
      </div>
    </div>
  );
});
