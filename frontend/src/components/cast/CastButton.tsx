/**
 * Cast button with device selection dropdown
 */

import { useState } from 'react';
import { Button } from '@heroui/button';
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem, DropdownSection } from '@heroui/dropdown';
import { Chip } from '@heroui/chip';
import { Spinner } from '@heroui/spinner';
import { IconCast, IconRefresh, IconPlus } from '@tabler/icons-react';
import { useCast } from '../../hooks/useCast';
import type { CastDevice, CastMediaInput } from '../../lib/graphql';

interface CastButtonProps {
  /** Media file ID to cast */
  mediaFileId: string;
  /** Episode ID (optional, for tracking) */
  episodeId?: string;
  /** Start position in seconds */
  startPosition?: number;
  /** Button size */
  size?: 'sm' | 'md' | 'lg';
  /** Button variant override */
  variant?: 'solid' | 'bordered' | 'light' | 'flat' | 'faded' | 'shadow' | 'ghost';
  /** Additional class names for the button */
  className?: string;
  /** Called when cast starts */
  onCastStart?: () => void;
  /** Called on error */
  onError?: (error: string) => void;
}

export function CastButton({
  mediaFileId,
  episodeId,
  startPosition,
  size = 'sm',
  variant,
  className,
  onCastStart,
  onError,
}: CastButtonProps) {
  const { devices, activeSession, isDiscovering, discoverDevices, castMedia } = useCast();
  const [isCasting, setIsCasting] = useState(false);

  const isCastingThisMedia = activeSession?.mediaFileId === mediaFileId;
  const hasDevices = devices.length > 0;

  const handleCast = async (device: CastDevice) => {
    setIsCasting(true);
    try {
      const input: CastMediaInput = {
        deviceId: device.id,
        mediaFileId,
        episodeId,
        startPosition,
      };
      const result = await castMedia(input);
      if (result.success) {
        onCastStart?.();
      } else {
        onError?.(result.error || 'Failed to cast');
      }
    } finally {
      setIsCasting(false);
    }
  };

  const getDeviceIcon = (device: CastDevice) => {
    if (device.isConnected || isCastingThisMedia) {
      return <IconCast size={16} className="text-primary" />;
    }
    return <IconCast size={16} className="text-default-400" />;
  };

  return (
    <Dropdown>
      <DropdownTrigger>
        <Button
          isIconOnly={size === 'sm'}
          size={size}
          variant={isCastingThisMedia ? 'solid' : (variant || 'flat')}
          color={isCastingThisMedia ? 'primary' : 'default'}
          className={className}
          isLoading={isCasting}
        >
          <IconCast size={size === 'lg' ? 24 : 20} />
          {size !== 'sm' && <span className="ml-2">Cast</span>}
        </Button>
      </DropdownTrigger>
      <DropdownMenu aria-label="Cast devices">
        <DropdownSection title="Cast to device" showDivider={hasDevices}>
          {hasDevices ? (
            devices.map((device) => (
              <DropdownItem
                key={device.id}
                startContent={getDeviceIcon(device)}
                description={device.model || device.address}
                onPress={() => handleCast(device)}
              >
                <div className="flex items-center gap-2">
                  {device.name}
                  {device.isFavorite && (
                    <Chip size="sm" variant="flat" color="warning">★</Chip>
                  )}
                </div>
              </DropdownItem>
            ))
          ) : (
            <DropdownItem key="no-devices" isReadOnly className="text-default-400">
              No devices found
            </DropdownItem>
          )}
        </DropdownSection>
        <DropdownSection>
          <DropdownItem
            key="discover"
            startContent={isDiscovering ? <Spinner size="sm" /> : <IconRefresh size={16} />}
            onPress={discoverDevices}
            isDisabled={isDiscovering}
          >
            {isDiscovering ? 'Discovering...' : 'Discover devices'}
          </DropdownItem>
          <DropdownItem
            key="manage"
            startContent={<IconPlus size={16} />}
            href="/settings/casting"
          >
            Manage devices
          </DropdownItem>
        </DropdownSection>
      </DropdownMenu>
    </Dropdown>
  );
}
