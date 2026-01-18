/**
 * Cast control bar - shows when actively casting
 */

import { Button } from '@heroui/button';
import { Progress } from '@heroui/progress';
import { Slider } from '@heroui/slider';
import { Tooltip } from '@heroui/tooltip';
import { Card, CardBody } from '@heroui/card';
import {
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerStop,
  IconCast,
  IconVolume,
  IconVolumeOff,
  IconX,
} from '@tabler/icons-react';
import { useCast } from '../../hooks/useCast';

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }
  return `${m}:${s.toString().padStart(2, '0')}`;
}

export function CastControlBar() {
  const { activeSession, play, pause, stop, seek, setVolume, setMuted } = useCast();

  if (!activeSession) return null;

  const isPlaying = activeSession.playerState === 'PLAYING';
  const isBuffering = activeSession.playerState === 'BUFFERING';
  const progress = activeSession.duration 
    ? (activeSession.currentTime / activeSession.duration) * 100 
    : 0;

  const handleSeek = (value: number | number[]) => {
    const position = Array.isArray(value) ? value[0] : value;
    if (activeSession.duration) {
      seek((position / 100) * activeSession.duration);
    }
  };

  const handleVolumeChange = (value: number | number[]) => {
    const vol = Array.isArray(value) ? value[0] : value;
    setVolume(vol / 100);
  };

  return (
    <Card className="fixed bottom-4 left-1/2 -translate-x-1/2 w-[90%] max-w-2xl z-50 shadow-lg">
      <CardBody className="p-3">
        <div className="flex items-center gap-4">
          {/* Cast indicator */}
          <div className="flex items-center gap-2 text-primary">
            <IconCast size={20} />
            <span className="text-sm font-medium truncate max-w-32">
              {activeSession.deviceName || 'Casting'}
            </span>
          </div>

          {/* Playback controls */}
          <div className="flex items-center gap-1">
            <Tooltip content={isPlaying ? 'Pause' : 'Play'}>
              <Button
                isIconOnly
                size="sm"
                variant="light"
                onPress={() => isPlaying ? pause() : play()}
                isDisabled={isBuffering}
              >
                {isPlaying ? <IconPlayerPause size={20} /> : <IconPlayerPlay size={20} />}
              </Button>
            </Tooltip>
            <Tooltip content="Stop">
              <Button
                isIconOnly
                size="sm"
                variant="light"
                color="danger"
                onPress={stop}
              >
                <IconPlayerStop size={20} />
              </Button>
            </Tooltip>
          </div>

          {/* Progress / seek */}
          <div className="flex-1 flex items-center gap-2">
            <span className="text-xs text-default-400 w-12 text-right">
              {formatTime(activeSession.currentTime)}
            </span>
            <Slider
              aria-label="Seek position"
              size="sm"
              step={0.1}
              maxValue={100}
              minValue={0}
              value={progress}
              onChange={handleSeek}
              className="flex-1"
              isDisabled={!activeSession.duration}
            />
            <span className="text-xs text-default-400 w-12">
              {activeSession.duration ? formatTime(activeSession.duration) : '--:--'}
            </span>
          </div>

          {/* Volume */}
          <div className="flex items-center gap-1 w-32">
            <Tooltip content={activeSession.isMuted ? 'Unmute' : 'Mute'}>
              <Button
                isIconOnly
                size="sm"
                variant="light"
                onPress={() => setMuted(!activeSession.isMuted)}
              >
                {activeSession.isMuted ? (
                  <IconVolumeOff size={18} />
                ) : (
                  <IconVolume size={18} />
                )}
              </Button>
            </Tooltip>
            <Slider
              aria-label="Volume"
              size="sm"
              step={1}
              maxValue={100}
              minValue={0}
              value={activeSession.isMuted ? 0 : activeSession.volume * 100}
              onChange={handleVolumeChange}
              className="flex-1"
            />
          </div>

          {/* Close */}
          <Tooltip content="Stop casting">
            <Button
              isIconOnly
              size="sm"
              variant="light"
              onPress={stop}
            >
              <IconX size={18} />
            </Button>
          </Tooltip>
        </div>

        {/* Buffering indicator */}
        {isBuffering && (
          <Progress
            size="sm"
            isIndeterminate
            color="primary"
            className="mt-2"
            aria-label="Buffering"
          />
        )}
      </CardBody>
    </Card>
  );
}
