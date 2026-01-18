/**
 * Cast settings page
 * 
 * Manage cast devices and global casting settings.
 */

import { useState, useEffect } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { Card, CardBody, CardHeader } from '@heroui/card';
import { Button } from '@heroui/button';
import { Input } from '@heroui/input';
import { Switch } from '@heroui/switch';
import { Slider } from '@heroui/slider';
import { Chip } from '@heroui/chip';
import { Spinner } from '@heroui/spinner';
import { Divider } from '@heroui/divider';
import { Table, TableHeader, TableColumn, TableBody, TableRow, TableCell } from '@heroui/table';
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, useDisclosure } from '@heroui/modal';
import { Tooltip } from '@heroui/tooltip';
import {
  IconCast,
  IconRefresh,
  IconPlus,
  IconTrash,
  IconStar,
  IconStarFilled,
} from '@tabler/icons-react';
import {
  graphqlClient,
  CAST_DEVICES_QUERY,
  CAST_SETTINGS_QUERY,
  DISCOVER_CAST_DEVICES_MUTATION,
  ADD_CAST_DEVICE_MUTATION,
  UPDATE_CAST_DEVICE_MUTATION,
  REMOVE_CAST_DEVICE_MUTATION,
  UPDATE_CAST_SETTINGS_MUTATION,
  type CastDevice,
  type CastSettings,
  type CastDeviceResult,
  type CastSettingsResult,
} from '../../lib/graphql';

export const Route = createFileRoute('/settings/casting')({
  component: CastingSettingsPage,
});

function CastingSettingsPage() {
  const [devices, setDevices] = useState<CastDevice[]>([]);
  const [settings, setSettings] = useState<CastSettings | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [isSavingSettings, setIsSavingSettings] = useState(false);

  // Add device modal
  const { isOpen, onOpen, onClose } = useDisclosure();
  const [newDeviceAddress, setNewDeviceAddress] = useState('');
  const [newDevicePort, setNewDevicePort] = useState('8009');
  const [newDeviceName, setNewDeviceName] = useState('');
  const [isAddingDevice, setIsAddingDevice] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);

  // Load data
  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setIsLoading(true);
    try {
      const [devicesRes, settingsRes] = await Promise.all([
        graphqlClient.query<{ castDevices: CastDevice[] }>(CAST_DEVICES_QUERY, {}).toPromise(),
        graphqlClient.query<{ castSettings: CastSettings }>(CAST_SETTINGS_QUERY, {}).toPromise(),
      ]);

      if (devicesRes.data?.castDevices) {
        setDevices(devicesRes.data.castDevices);
      }
      if (settingsRes.data?.castSettings) {
        setSettings(settingsRes.data.castSettings);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleDiscover = async () => {
    setIsDiscovering(true);
    try {
      const result = await graphqlClient
        .mutation<{ discoverCastDevices: CastDevice[] }>(DISCOVER_CAST_DEVICES_MUTATION, {})
        .toPromise();
      if (result.data?.discoverCastDevices) {
        setDevices(result.data.discoverCastDevices);
      }
    } finally {
      setIsDiscovering(false);
    }
  };

  const handleAddDevice = async () => {
    if (!newDeviceAddress.trim()) {
      setAddError('IP address is required');
      return;
    }

    setIsAddingDevice(true);
    setAddError(null);

    try {
      const result = await graphqlClient
        .mutation<{ addCastDevice: CastDeviceResult }>(ADD_CAST_DEVICE_MUTATION, {
          input: {
            address: newDeviceAddress.trim(),
            port: newDevicePort ? parseInt(newDevicePort) : undefined,
            name: newDeviceName.trim() || undefined,
          },
        })
        .toPromise();

      if (result.data?.addCastDevice.success && result.data.addCastDevice.device) {
        setDevices((prev) => [...prev, result.data!.addCastDevice.device!]);
        onClose();
        setNewDeviceAddress('');
        setNewDevicePort('8009');
        setNewDeviceName('');
      } else {
        setAddError(result.data?.addCastDevice.error || 'Failed to add device');
      }
    } catch (e) {
      setAddError(e instanceof Error ? e.message : 'Failed to add device');
    } finally {
      setIsAddingDevice(false);
    }
  };

  const handleToggleFavorite = async (device: CastDevice) => {
    try {
      const result = await graphqlClient
        .mutation<{ updateCastDevice: CastDeviceResult }>(UPDATE_CAST_DEVICE_MUTATION, {
          id: device.id,
          input: { isFavorite: !device.isFavorite },
        })
        .toPromise();

      if (result.data?.updateCastDevice.success && result.data.updateCastDevice.device) {
        setDevices((prev) =>
          prev.map((d) => (d.id === device.id ? result.data!.updateCastDevice.device! : d))
        );
      }
    } catch (e) {
      console.error('Failed to toggle favorite:', e);
    }
  };

  const handleRemoveDevice = async (deviceId: string) => {
    try {
      const result = await graphqlClient
        .mutation<{ removeCastDevice: { success: boolean; error?: string } }>(
          REMOVE_CAST_DEVICE_MUTATION,
          { id: deviceId }
        )
        .toPromise();

      if (result.data?.removeCastDevice.success) {
        setDevices((prev) => prev.filter((d) => d.id !== deviceId));
      }
    } catch (e) {
      console.error('Failed to remove device:', e);
    }
  };

  const handleUpdateSettings = async (updates: Partial<CastSettings>) => {
    if (!settings) return;

    setIsSavingSettings(true);
    try {
      const result = await graphqlClient
        .mutation<{ updateCastSettings: CastSettingsResult }>(UPDATE_CAST_SETTINGS_MUTATION, {
          input: updates,
        })
        .toPromise();

      if (result.data?.updateCastSettings.success && result.data.updateCastSettings.settings) {
        setSettings(result.data.updateCastSettings.settings);
      }
    } finally {
      setIsSavingSettings(false);
    }
  };

  const getDeviceTypeLabel = (deviceType: string) => {
    switch (deviceType) {
      case 'CHROMECAST':
        return 'Chromecast';
      case 'CHROMECAST_AUDIO':
        return 'Chromecast Audio';
      case 'GOOGLE_HOME':
        return 'Google Home';
      case 'GOOGLE_NEST_HUB':
        return 'Nest Hub';
      case 'ANDROID_TV':
        return 'Android TV';
      default:
        return 'Unknown';
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Spinner size="lg" />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">Casting</h2>
          <p className="text-default-500 text-sm">
            Manage Chromecast and Google Cast devices
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            variant="flat"
            startContent={isDiscovering ? <Spinner size="sm" /> : <IconRefresh size={16} />}
            onPress={handleDiscover}
            isDisabled={isDiscovering}
          >
            {isDiscovering ? 'Discovering...' : 'Discover'}
          </Button>
          <Button color="primary" startContent={<IconPlus size={16} />} onPress={onOpen}>
            Add Device
          </Button>
        </div>
      </div>

      {/* Devices Section */}
      <Card>
        <CardHeader>
          <p className="font-semibold">Cast Devices</p>
        </CardHeader>
        <Divider />
        <CardBody>
          {devices.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <IconCast size={48} className="text-default-300 mb-4" />
              <p className="text-default-500 mb-2">No cast devices found</p>
              <p className="text-small text-default-400 mb-4">
                Click &quot;Discover&quot; to scan for devices on your network, or add one manually.
              </p>
              <Button variant="flat" startContent={<IconRefresh size={16} />} onPress={handleDiscover}>
                Discover Devices
              </Button>
            </div>
          ) : (
            <Table aria-label="Cast devices" removeWrapper>
              <TableHeader>
                <TableColumn>NAME</TableColumn>
                <TableColumn>TYPE</TableColumn>
                <TableColumn>ADDRESS</TableColumn>
                <TableColumn>STATUS</TableColumn>
                <TableColumn width={120}>ACTIONS</TableColumn>
              </TableHeader>
              <TableBody>
                {devices.map((device) => (
                  <TableRow key={device.id}>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <IconCast size={18} className="text-default-400" />
                        <span className="font-medium">{device.name}</span>
                        {device.isManual && (
                          <Chip size="sm" variant="flat">
                            Manual
                          </Chip>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <Chip size="sm" variant="flat" color="primary">
                        {getDeviceTypeLabel(device.deviceType)}
                      </Chip>
                    </TableCell>
                    <TableCell>
                      <code className="text-small">
                        {device.address}:{device.port}
                      </code>
                    </TableCell>
                    <TableCell>
                      {device.isConnected ? (
                        <Chip size="sm" color="success" variant="flat">
                          Connected
                        </Chip>
                      ) : device.lastSeenAt ? (
                        <span className="text-small text-default-400">
                          Last seen: {new Date(device.lastSeenAt).toLocaleString()}
                        </span>
                      ) : (
                        <span className="text-small text-default-400">Never seen</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Tooltip content={device.isFavorite ? 'Remove from favorites' : 'Add to favorites'}>
                          <Button
                            isIconOnly
                            size="sm"
                            variant="light"
                            onPress={() => handleToggleFavorite(device)}
                          >
                            {device.isFavorite ? (
                              <IconStarFilled size={16} className="text-warning" />
                            ) : (
                              <IconStar size={16} />
                            )}
                          </Button>
                        </Tooltip>
                        <Tooltip content="Remove device">
                          <Button
                            isIconOnly
                            size="sm"
                            variant="light"
                            color="danger"
                            onPress={() => handleRemoveDevice(device.id)}
                          >
                            <IconTrash size={16} />
                          </Button>
                        </Tooltip>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardBody>
      </Card>

      {/* Settings Section */}
      {settings && (
        <Card>
          <CardHeader>
            <p className="font-semibold">Cast Settings</p>
          </CardHeader>
          <Divider />
          <CardBody className="flex flex-col gap-6">
            <div className="flex justify-between items-center">
              <div>
                <p className="font-medium">Auto-Discovery</p>
                <p className="text-small text-default-400">
                  Automatically scan for cast devices on the network
                </p>
              </div>
              <Switch
                isSelected={settings.autoDiscoveryEnabled}
                onValueChange={(value) => handleUpdateSettings({ autoDiscoveryEnabled: value })}
                isDisabled={isSavingSettings}
              />
            </div>

            <div className="flex justify-between items-center">
              <div>
                <p className="font-medium">Discovery Interval</p>
                <p className="text-small text-default-400">
                  How often to scan for new devices (seconds)
                </p>
              </div>
              <Input
                type="number"
                className="w-24"
                value={settings.discoveryIntervalSeconds.toString()}
                onValueChange={(value) =>
                  handleUpdateSettings({ discoveryIntervalSeconds: parseInt(value) || 30 })
                }
                isDisabled={isSavingSettings || !settings.autoDiscoveryEnabled}
                min={10}
                max={300}
              />
            </div>

            <div className="flex flex-col gap-2">
              <div className="flex justify-between items-center">
                <div>
                  <p className="font-medium">Default Volume</p>
                  <p className="text-small text-default-400">
                    Volume level when starting a new cast session
                  </p>
                </div>
                <span className="text-default-500">{Math.round(settings.defaultVolume * 100)}%</span>
              </div>
              <Slider
                aria-label="Default volume"
                step={5}
                minValue={0}
                maxValue={100}
                value={settings.defaultVolume * 100}
                onChange={(value: number | number[]) => {
                  const v = Array.isArray(value) ? value[0] : value;
                  handleUpdateSettings({ defaultVolume: v / 100 });
                }}
                isDisabled={isSavingSettings}
              />
            </div>

            <div className="flex justify-between items-center">
              <div>
                <p className="font-medium">Transcode Incompatible Files</p>
                <p className="text-small text-default-400">
                  Automatically transcode files that aren&apos;t compatible with Chromecast
                </p>
              </div>
              <Switch
                isSelected={settings.transcodeIncompatible}
                onValueChange={(value) => handleUpdateSettings({ transcodeIncompatible: value })}
                isDisabled={isSavingSettings}
              />
            </div>
          </CardBody>
        </Card>
      )}

      {/* Add Device Modal */}
      <Modal isOpen={isOpen} onClose={onClose}>
        <ModalContent>
          <ModalHeader>Add Cast Device</ModalHeader>
          <ModalBody>
            <div className="flex flex-col gap-4">
              <Input
                label="IP Address"
                placeholder="192.168.1.100"
                value={newDeviceAddress}
                onValueChange={setNewDeviceAddress}
                isRequired
                isInvalid={!!addError}
                errorMessage={addError}
              />
              <Input
                label="Port"
                placeholder="8009"
                value={newDevicePort}
                onValueChange={setNewDevicePort}
                type="number"
                description="Default Chromecast port is 8009"
              />
              <Input
                label="Name (optional)"
                placeholder="Living Room TV"
                value={newDeviceName}
                onValueChange={setNewDeviceName}
                description="A friendly name for this device"
              />
            </div>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onClose}>
              Cancel
            </Button>
            <Button
              color="primary"
              onPress={handleAddDevice}
              isLoading={isAddingDevice}
            >
              Add Device
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  );
}
