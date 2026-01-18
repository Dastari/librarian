/**
 * Volume control component
 * 
 * Click to mute/unmute, hover to show volume slider.
 */

import { useState, useRef, useEffect } from 'react';
import { Button } from '@heroui/button';
import { Slider } from '@heroui/slider';
import { IconVolume, IconVolumeOff, IconVolume2 } from '@tabler/icons-react';

interface VolumeControlProps {
  /** Current volume (0-1) */
  volume: number;
  /** Whether audio is muted */
  isMuted: boolean;
  /** Called when volume changes */
  onVolumeChange: (volume: number) => void;
  /** Called when mute is toggled */
  onMuteToggle: () => void;
  /** Button size */
  size?: 'sm' | 'md' | 'lg';
  /** Icon size in pixels */
  iconSize?: number;
  /** Additional button class */
  className?: string;
}

export function VolumeControl({
  volume,
  isMuted,
  onVolumeChange,
  onMuteToggle,
  size = 'sm',
  iconSize = 18,
  className = '',
}: VolumeControlProps) {
  const [isHovered, setIsHovered] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const hideTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Clear timeout on unmount
  useEffect(() => {
    return () => {
      if (hideTimeoutRef.current) clearTimeout(hideTimeoutRef.current);
    };
  }, []);

  const handleMouseEnter = () => {
    if (hideTimeoutRef.current) clearTimeout(hideTimeoutRef.current);
    setIsHovered(true);
  };

  const handleMouseLeave = () => {
    // Delay hiding to allow moving to the slider
    hideTimeoutRef.current = setTimeout(() => {
      setIsHovered(false);
    }, 300);
  };

  const handleSliderChange = (value: number | number[]) => {
    const newVolume = Array.isArray(value) ? value[0] : value;
    onVolumeChange(newVolume / 100);
  };

  // Determine which icon to show
  const VolumeIcon = isMuted ? IconVolumeOff : volume < 0.5 ? IconVolume2 : IconVolume;

  return (
    <div 
      ref={containerRef}
      className="relative"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <Button 
        isIconOnly 
        size={size} 
        variant="light" 
        className={className}
        onPress={onMuteToggle}
      >
        <VolumeIcon size={iconSize} />
      </Button>

      {/* Volume slider popup */}
      {isHovered && (
        <div 
          className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 p-3 bg-default-100 rounded-lg shadow-lg"
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
          <Slider
            size="sm"
            step={1}
            minValue={0}
            maxValue={100}
            value={isMuted ? 0 : Math.round(volume * 100)}
            onChange={handleSliderChange}
            orientation="vertical"
            className="h-24"
            aria-label="Volume"
          />
        </div>
      )}
    </div>
  );
}
