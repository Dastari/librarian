/**
 * Cast settings page
 *
 * Manage cast devices and global casting settings.
 */

import { useState, useEffect } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { Card, CardBody, CardHeader } from "@heroui/card";
import { Button } from "@heroui/button";
import { Input } from "@heroui/input";
import { Switch } from "@heroui/switch";
import { Slider } from "@heroui/slider";
import { Chip } from "@heroui/chip";
import { Spinner } from "@heroui/spinner";
import { Divider } from "@heroui/divider";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  useDisclosure,
} from "@heroui/modal";
import {
  IconCast,
  IconRefresh,
  IconPlus,
  IconTrash,
  IconStar,
  IconStarFilled,
  IconPlugConnected,
  IconPencil,
} from "@tabler/icons-react";
import { DataTable, type DataTableColumn, type RowAction } from "../../components/data-table";
import {
  CastDevicesDocument,
  CastSettingsDocument,
  DiscoverCastDevicesOpDocument,
  CreateCastDeviceDocument,
  UpdateCastDeviceDocument,
  DeleteCastDeviceDocument,
  CreateCastSettingDocument,
  UpdateCastSettingDocument,
  type CreateCastDeviceMutation,
  type CreateCastDeviceMutationVariables,
  type UpdateCastDeviceMutation,
  type UpdateCastDeviceMutationVariables,
  type DeleteCastDeviceMutation,
  type DeleteCastDeviceMutationVariables,
  type CreateCastSettingMutation,
  type CreateCastSettingMutationVariables,
  type UpdateCastSettingMutation,
  type UpdateCastSettingMutationVariables,
  type DiscoverCastDevicesOpMutationVariables,
  type DiscoverCastDevicesOpMutation,
  type CastDevicesQuery,
  type CastSettingsQuery,
} from "../../lib/graphql/generated/graphql";
import { apolloClient, useMutation } from "../../lib/graphql/client";

export const Route = createFileRoute("/settings/casting")({
  component: CastingSettingsPage,
});

function CastingSettingsPage() {
  type CastDevice =
    DiscoverCastDevicesOpMutation["DiscoverCastDevices"][number];
  type CastSettings = {
    autoDiscoveryEnabled: boolean;
    discoveryIntervalSeconds: number;
    defaultVolume: number;
    transcodeIncompatible: boolean;
    preferredQuality: string | null;
  };

  const [devices, setDevices] = useState<CastDevice[]>([]);
  const [settings, setSettings] = useState<CastSettings | null>(null);
  const [settingsId, setSettingsId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [isSavingSettings, setIsSavingSettings] = useState(false);

  // Add device modal
  const { isOpen, onOpen, onClose } = useDisclosure();
  const [newDeviceAddress, setNewDeviceAddress] = useState("");
  const [newDevicePort, setNewDevicePort] = useState("8009");
  const [newDeviceName, setNewDeviceName] = useState("");
  const [isAddingDevice, setIsAddingDevice] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);
  const [discoverCastDevices] = useMutation<
    DiscoverCastDevicesOpMutation,
    DiscoverCastDevicesOpMutationVariables
  >(DiscoverCastDevicesOpDocument);
  const [createCastDevice] = useMutation<
    CreateCastDeviceMutation,
    CreateCastDeviceMutationVariables
  >(CreateCastDeviceDocument);
  const [updateCastDevice] = useMutation<
    UpdateCastDeviceMutation,
    UpdateCastDeviceMutationVariables
  >(UpdateCastDeviceDocument);
  const [deleteCastDevice] = useMutation<
    DeleteCastDeviceMutation,
    DeleteCastDeviceMutationVariables
  >(DeleteCastDeviceDocument);
  const [createCastSetting] = useMutation<
    CreateCastSettingMutation,
    CreateCastSettingMutationVariables
  >(CreateCastSettingDocument);
  const [updateCastSetting] = useMutation<
    UpdateCastSettingMutation,
    UpdateCastSettingMutationVariables
  >(UpdateCastSettingDocument);

  const mapDeviceNode = (
    node: CastDevicesQuery["CastDevices"]["Edges"][0]["Node"],
  ): CastDevice => ({
    id: node.Id,
    name: node.Name,
    address: node.Address,
    port: node.Port,
    model: node.Model ?? null,
    deviceType: node.DeviceType as CastDevice["deviceType"],
    isFavorite: node.IsFavorite,
    isManual: node.IsManual,
    isConnected: false,
    lastSeenAt: node.LastSeenAt ?? null,
  });

  const mapSettingNode = (
    node: CastSettingsQuery["CastSettings"]["Edges"][0]["Node"],
  ): CastSettings => ({
    autoDiscoveryEnabled: node.AutoDiscoveryEnabled,
    discoveryIntervalSeconds: node.DiscoveryIntervalSeconds,
    defaultVolume: node.DefaultVolume,
    transcodeIncompatible: node.TranscodeIncompatible,
    preferredQuality: node.PreferredQuality ?? null,
  });

  // Load data
  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setIsLoading(true);
    try {
      const [devicesRes, settingsRes] = await Promise.all([
        apolloClient.query({
          query: CastDevicesDocument,
          fetchPolicy: "network-only",
        }),
        apolloClient.query({
          query: CastSettingsDocument,
          variables: {
            Page: { Limit: 1, Offset: 0 },
            OrderBy: [{ UpdatedAt: "Desc" }],
          },
          fetchPolicy: "network-only",
        }),
      ]);

      if (devicesRes.data?.CastDevices?.Edges) {
        setDevices(
          devicesRes.data.CastDevices.Edges.map((edge) =>
            mapDeviceNode(edge.Node),
          ),
        );
      }
      if (settingsRes.data?.CastSettings?.Edges?.length) {
        const node = settingsRes.data.CastSettings.Edges[0].Node;
        setSettings(mapSettingNode(node));
        setSettingsId(node.Id);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleDiscover = async () => {
    setIsDiscovering(true);
    try {
      const result = await discoverCastDevices();
      if (result.data?.DiscoverCastDevices) {
        setDevices(
          result.data.DiscoverCastDevices.map((device) => ({
            id: device.id,
            name: device.name,
            address: device.address,
            port: device.port,
            model: device.model ?? null,
            deviceType: device.deviceType as CastDevice["deviceType"],
            isFavorite: device.isFavorite,
            isManual: device.isManual,
            isConnected: device.isConnected ?? false,
            lastSeenAt: device.lastSeenAt ?? null,
          })),
        );
      }
    } finally {
      setIsDiscovering(false);
    }
  };

  const handleAddDevice = async () => {
    if (!newDeviceAddress.trim()) {
      setAddError("IP address is required");
      return;
    }

    setIsAddingDevice(true);
    setAddError(null);

    try {
      const result = await createCastDevice({
        variables: {
          Input: {
            Name:
              newDeviceName.trim() ||
              `Cast Device (${newDeviceAddress.trim()})`,
            Address: newDeviceAddress.trim(),
            Port: newDevicePort ? parseInt(newDevicePort, 10) : 8009,
            Model: null,
            DeviceType: "CHROMECAST",
            IsFavorite: false,
            IsManual: true,
            LastSeenAt: null,
          },
        },
      });

      const payload = result.data?.CreateCastDevice;
      if (payload?.Success && payload.CastDevice) {
        setDevices((prev) => [
          ...prev,
          mapDeviceNode(
            payload.CastDevice as CastDevicesQuery["CastDevices"]["Edges"][0]["Node"],
          ),
        ]);
        onClose();
        setNewDeviceAddress("");
        setNewDevicePort("8009");
        setNewDeviceName("");
      } else {
        setAddError(payload?.Error || "Failed to add device");
      }
    } catch (e) {
      setAddError(e instanceof Error ? e.message : "Failed to add device");
    } finally {
      setIsAddingDevice(false);
    }
  };

  const handleToggleFavorite = async (device: CastDevice) => {
    try {
      const result = await updateCastDevice({
        variables: {
          Id: device.id,
          Input: {
            IsFavorite: !device.isFavorite,
          },
        },
      });
      if (
        result.data?.UpdateCastDevice.Success &&
        result.data.UpdateCastDevice.CastDevice
      ) {
        setDevices((prev) =>
          prev.map((d) =>
            d.id === device.id
              ? mapDeviceNode(
                  result.data!.UpdateCastDevice
                    .CastDevice as CastDevicesQuery["CastDevices"]["Edges"][0]["Node"],
                )
              : d,
          ),
        );
      }
    } catch (e) {
      console.error("Failed to toggle favorite:", e);
    }
  };

  const handleRemoveDevice = async (deviceId: string) => {
    try {
      const result = await deleteCastDevice({ variables: { Id: deviceId } });
      if (result.data?.DeleteCastDevice.Success) {
        setDevices((prev) => prev.filter((d) => d.id !== deviceId));
      }
    } catch (e) {
      console.error("Failed to remove device:", e);
    }
  };

  const handleUpdateSettings = async (updates: Partial<CastSettings>) => {
    if (!settings) return;

    setIsSavingSettings(true);
    try {
      if (settingsId) {
        const result = await updateCastSetting({
          variables: {
            Id: settingsId,
            Input: {
              AutoDiscoveryEnabled: updates.autoDiscoveryEnabled,
              DiscoveryIntervalSeconds: updates.discoveryIntervalSeconds,
              DefaultVolume: updates.defaultVolume,
              TranscodeIncompatible: updates.transcodeIncompatible,
              PreferredQuality: updates.preferredQuality ?? undefined,
            },
          },
        });
        if (
          result.data?.UpdateCastSetting.Success &&
          result.data.UpdateCastSetting.CastSetting
        ) {
          setSettings(
            mapSettingNode(
              result.data.UpdateCastSetting
                .CastSetting as CastSettingsQuery["CastSettings"]["Edges"][0]["Node"],
            ),
          );
        }
      } else {
        const result = await createCastSetting({
          variables: {
            Input: {
              AutoDiscoveryEnabled: updates.autoDiscoveryEnabled ?? true,
              DiscoveryIntervalSeconds: updates.discoveryIntervalSeconds ?? 30,
              DefaultVolume: updates.defaultVolume ?? 1,
              TranscodeIncompatible: updates.transcodeIncompatible ?? false,
              PreferredQuality: updates.preferredQuality ?? undefined,
            },
          },
        });
        if (
          result.data?.CreateCastSetting.Success &&
          result.data.CreateCastSetting.CastSetting
        ) {
          setSettings(
            mapSettingNode(
              result.data.CreateCastSetting
                .CastSetting as CastSettingsQuery["CastSettings"]["Edges"][0]["Node"],
            ),
          );
          setSettingsId(result.data.CreateCastSetting.CastSetting.Id);
        }
      }
    } catch (e) {
      console.error("Failed to update cast settings:", e);
    } finally {
      setIsSavingSettings(false);
    }
  };

  const getDeviceTypeLabel = (deviceType: string) => {
    switch (deviceType) {
      case "CHROMECAST":
        return "Chromecast";
      case "CHROMECAST_AUDIO":
        return "Chromecast Audio";
      case "GOOGLE_HOME":
        return "Google Home";
      case "GOOGLE_NEST_HUB":
        return "Nest Hub";
      case "ANDROID_TV":
        return "Android TV";
      default:
        return "Unknown";
    }
  };

  const deviceColumns: DataTableColumn<CastDevice>[] = [
    {
      key: "name",
      label: "Name",
      sortable: true,
      render: (device) => (
        <div className="flex items-center gap-2">
          <IconCast size={18} className="text-default-400" />
          <span className="font-medium">{device.name}</span>
          {device.isManual ? (
            <Chip size="sm" variant="flat">
              Manual
            </Chip>
          ) : null}
        </div>
      ),
    },
    {
      key: "deviceType",
      label: "Type",
      sortable: true,
      width: 180,
      render: (device) => (
        <Chip size="sm" variant="flat" color="primary">
          {getDeviceTypeLabel(device.deviceType)}
        </Chip>
      ),
    },
    {
      key: "address",
      label: "Address",
      sortable: true,
      width: 220,
      render: (device) => (
        <code className="text-small">
          {device.address}:{device.port}
        </code>
      ),
    },
    {
      key: "status",
      label: "Status",
      width: 220,
      render: (device) =>
        device.isConnected ? (
          <Chip size="sm" color="success" variant="flat">
            Connected
          </Chip>
        ) : device.lastSeenAt ? (
          <span className="text-small text-default-400">
            Last seen: {new Date(device.lastSeenAt).toLocaleString()}
          </span>
        ) : (
          <span className="text-small text-default-400">Never seen</span>
        ),
    },
  ];

  const deviceActions: RowAction<CastDevice>[] = [
    {
      key: "favorite",
      label: (device) =>
        device.isFavorite ? "Remove from favorites" : "Add to favorites",
      icon: (device) =>
        device.isFavorite ? (
          <IconStarFilled size={16} className="text-warning" />
        ) : (
          <IconStar size={16} />
        ),
      inDropdown: false,
      onAction: (device) => void handleToggleFavorite(device),
    },
    {
      key: "test",
      label: "Discover",
      icon: <IconPlugConnected size={16} className="text-blue-400" />,
      inDropdown: false,
      onAction: () => void handleDiscover(),
      isDisabled: () => isDiscovering,
    },
    {
      key: "edit",
      label: "Edit",
      icon: <IconPencil size={16} className="text-default-400" />,
      inDropdown: false,
      onAction: () => onOpen(),
    },
    {
      key: "remove",
      label: "Remove device",
      icon: <IconTrash size={16} className="text-red-400" />,
      isDestructive: true,
      inDropdown: false,
      onAction: (device) => void handleRemoveDevice(device.id),
    },
  ];

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
            startContent={
              isDiscovering ? <Spinner size="sm" /> : <IconRefresh size={16} />
            }
            onPress={handleDiscover}
            isDisabled={isDiscovering}
          >
            {isDiscovering ? "Discovering..." : "Discover"}
          </Button>
          <Button
            color="primary"
            startContent={<IconPlus size={16} />}
            onPress={onOpen}
          >
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
          <DataTable
            stateKey="settings-cast-devices"
            data={devices}
            columns={deviceColumns}
            rowActions={deviceActions}
            getRowKey={(device) => device.id}
            ariaLabel="Cast devices"
            searchPlaceholder="Search cast devices..."
            showItemCount
            emptyContent={
              <div className="flex flex-col items-center justify-center py-12 text-center">
                <IconCast size={48} className="text-default-300 mb-4" />
                <p className="text-default-500 mb-2">No cast devices found</p>
                <p className="text-small text-default-400 mb-4">
                  Click &quot;Discover&quot; to scan for devices on your network,
                  or add one manually.
                </p>
                <Button
                  variant="flat"
                  startContent={<IconRefresh size={16} />}
                  onPress={handleDiscover}
                >
                  Discover Devices
                </Button>
              </div>
            }
          />
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
                onValueChange={(value) =>
                  handleUpdateSettings({ autoDiscoveryEnabled: value })
                }
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
                variant="flat"
                className="w-24"
                value={settings.discoveryIntervalSeconds.toString()}
                onValueChange={(value) =>
                  handleUpdateSettings({
                    discoveryIntervalSeconds: parseInt(value) || 30,
                  })
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
                <span className="text-default-500">
                  {Math.round(settings.defaultVolume * 100)}%
                </span>
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
                  Automatically transcode files that aren&apos;t compatible with
                  Chromecast
                </p>
              </div>
              <Switch
                isSelected={settings.transcodeIncompatible}
                onValueChange={(value) =>
                  handleUpdateSettings({ transcodeIncompatible: value })
                }
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
                labelPlacement="inside"
                variant="flat"
                placeholder="192.168.1.100"
                value={newDeviceAddress}
                onValueChange={setNewDeviceAddress}
                isRequired
                isInvalid={!!addError}
                errorMessage={addError}
                classNames={{
                  label: "text-sm font-medium text-primary!",
                }}
              />
              <Input
                label="Port"
                labelPlacement="inside"
                variant="flat"
                placeholder="8009"
                value={newDevicePort}
                onValueChange={setNewDevicePort}
                type="number"
                description="Default Chromecast port is 8009"
                classNames={{
                  label: "text-sm font-medium text-primary!",
                }}
              />
              <Input
                label="Name (optional)"
                labelPlacement="inside"
                variant="flat"
                placeholder="Living Room TV"
                value={newDeviceName}
                onValueChange={setNewDeviceName}
                description="A friendly name for this device"
                classNames={{
                  label: "text-sm font-medium text-primary!",
                }}
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
