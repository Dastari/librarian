import { useMutation, useQuery, useSubscription } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { IconCast, IconPlus, IconRadar, IconStar, IconStarFilled, IconTrash } from "@tabler/icons-react";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, ConfirmDialog, DataTable, type DataTableColumn, type DataTableRowAction, Dialog, EmptyState, FieldGroup, FormNumberField, FormSwitchField, FormTextField, Panel, StatusChip } from "@/components/ui";
import {
  AddCastDeviceDocument,
  DiscoverCastDevicesDocument,
  EntityCastDeviceChangedDocument,
  EntityCastDeviceListDocument,
  EntityCastDeviceUpdateDocument,
  EntityCastSettingCreateDocument,
  EntityCastSettingListDocument,
  EntityCastSettingUpdateDocument,
  RemoveCastDeviceDocument,
  type CastDeviceFieldsFragment,
} from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { SettingsForm } from "./SettingsForm";

const settingsSchema = z.object({ autoDiscoveryEnabled: z.boolean(), discoveryIntervalSeconds: z.number().int().min(30).max(3600), defaultVolume: z.number().min(0).max(1), transcodeIncompatible: z.boolean(), preferredQuality: z.string().trim() });
type SettingsValues = z.infer<typeof settingsSchema>;
const deviceSchema = z.object({ name: z.string().trim(), address: z.string().trim().min(1, "Enter the device address"), port: z.number().int().min(1).max(65535) });
type DeviceValues = z.infer<typeof deviceSchema>;

export function CastingSettings() {
  const isAdmin = useIsAdmin();
  const devices = useQuery(EntityCastDeviceListDocument, { variables: { orderBy: [{ name: "ASC" }], page: { limit: 100, offset: 0 } } });
  useSubscription(EntityCastDeviceChangedDocument, { onData: () => void devices.refetch() });
  const rows = (devices.data ?? devices.previousData)?.castDevices.edges.map((edge) => edge.node) ?? [];
  const settings = useQuery(EntityCastSettingListDocument, { variables: { page: { limit: 1, offset: 0 } } });
  const setting = settings.data?.castSettings.edges[0]?.node;

  const [discover, { loading: discovering }] = useMutation(DiscoverCastDevicesDocument);
  const [addDevice, { loading: adding }] = useMutation(AddCastDeviceDocument);
  const [removeDevice, { loading: removingBusy }] = useMutation(RemoveCastDeviceDocument);
  const [updateDevice] = useMutation(EntityCastDeviceUpdateDocument);
  const [createSetting] = useMutation(EntityCastSettingCreateDocument);
  const [updateSetting] = useMutation(EntityCastSettingUpdateDocument);
  const [addingOpen, setAddingOpen] = useState(false);
  const [removing, setRemoving] = useState<CastDeviceFieldsFragment | null>(null);

  const settingsForm = useForm<SettingsValues>({ resolver: zodResolver(settingsSchema), defaultValues: { autoDiscoveryEnabled: true, discoveryIntervalSeconds: 300, defaultVolume: 0.8, transcodeIncompatible: true, preferredQuality: "" } });
  useEffect(() => {
    if (setting) settingsForm.reset({ autoDiscoveryEnabled: setting.autoDiscoveryEnabled, discoveryIntervalSeconds: setting.discoveryIntervalSeconds, defaultVolume: setting.defaultVolume, transcodeIncompatible: setting.transcodeIncompatible, preferredQuality: setting.preferredQuality ?? "" });
  }, [setting, settingsForm]);
  const deviceForm = useForm<DeviceValues>({ resolver: zodResolver(deviceSchema), defaultValues: { name: "", address: "", port: 8009 } });

  const runDiscovery = async () => {
    try {
      const { data } = await discover();
      toast.success(`Found ${data?.discoverCastDevices.length ?? 0} devices`);
      void devices.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const submitDevice = deviceForm.handleSubmit(async (values) => {
    try {
      assertSuccess((await addDevice({ variables: { input: { address: values.address, port: values.port, name: values.name || null } } })).data?.addCastDevice, "Could not add device");
      toast.success("Device added");
      setAddingOpen(false);
      deviceForm.reset();
      void devices.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  });

  const columns: Array<DataTableColumn<CastDeviceFieldsFragment>> = [
    { id: "name", header: "Device", cell: (device) => (<span className="flex items-center gap-2">{device.isFavorite ? <IconStarFilled size={14} className="text-brand" /> : null}<span className="min-w-0"><span className="block truncate text-body-sm text-foreground">{device.name}</span><span className="block truncate font-mono text-label-sm text-muted">{device.address}:{device.port}{device.model ? ` · ${device.model}` : ""}</span></span></span>) },
    { id: "type", header: "Type", size: 130, cell: (device) => <span className="text-muted">{device.deviceType}</span> },
    { id: "playback", header: "Playback", size: 150, cell: (device) => (device.playbackSupported ? <StatusChip status={{ label: "Supported", tone: "success", dot: "bg-success" }} /> : <StatusChip status={{ label: "Discovery only", tone: "default", dot: "bg-muted" }} />) },
    { id: "seen", header: "Seen", size: 140, hideBelow: "md", cell: (device) => <span className="text-muted">{device.lastSeenAt ? formatRelative(device.lastSeenAt) : device.isManual ? "Manual" : "—"}</span> },
    { id: "probe", header: "Last probe", size: 200, hideBelow: "lg", cell: (device) => <span className={device.lastProbeError ? "text-danger" : "text-muted"}>{device.lastProbeError ?? (device.lastProbeAt ? formatRelative(device.lastProbeAt) : "—")}</span> },
  ];
  const actions: Array<DataTableRowAction<CastDeviceFieldsFragment>> = isAdmin
    ? [
        { key: "favorite", label: (device) => (device.isFavorite ? "Remove favourite" : "Favourite"), icon: <IconStar size={16} />, onAction: async (device) => { try { await updateDevice({ variables: { id: device.id, input: { isFavorite: !device.isFavorite } } }); } catch (error) { toast.danger(errorMessage(error)); } } },
        { key: "toggle", label: (device) => (device.enabled === false ? "Enable" : "Disable"), onAction: async (device) => { try { await updateDevice({ variables: { id: device.id, input: { enabled: device.enabled === false } } }); } catch (error) { toast.danger(errorMessage(error)); } } },
        { key: "remove", label: "Remove", icon: <IconTrash size={16} />, destructive: true, onAction: (device) => setRemoving(device) },
      ]
    : [];

  return (
    <div className="flex flex-col gap-6">
      <Panel title="Cast devices" description="Chromecast and Google TV receivers on your network. DLNA renderers are listed for information only." flush actions={<><Button size="sm" variant="ghost" onPress={() => void runDiscovery()} isPending={discovering}><IconRadar size={16} /> Discover</Button>{isAdmin ? <Button size="sm" variant="primary" onPress={() => setAddingOpen(true)}><IconPlus size={16} /> Add manually</Button> : null}</>}>
        <DataTable<CastDeviceFieldsFragment> className="px-4 pb-4" frame={false} columns={columns} rows={rows} getRowId={(device) => device.id} isLoading={devices.loading && rows.length === 0} rowActions={actions} noun="devices" emptyState={<EmptyState compact icon={IconCast} title="No devices found" description="Run discovery on the same network as your receivers, or add one by address." />} />
      </Panel>

      {isAdmin ? (
        <SettingsForm
          form={settingsForm}
          onSave={async (values) => {
            const input = { ...values, preferredQuality: values.preferredQuality || null };
            if (setting) assertSuccess((await updateSetting({ variables: { id: setting.id, input } })).data?.updateCastSetting, "Could not save");
            else assertSuccess((await createSetting({ variables: { input } })).data?.createCastSetting, "Could not save");
            void settings.refetch();
          }}
        >
          <Panel title="Casting behaviour">
            <FieldGroup columns={2}>
              <FormSwitchField control={settingsForm.control} name="autoDiscoveryEnabled" label="Discover devices automatically" className="sm:col-span-2" />
              <FormNumberField control={settingsForm.control} name="discoveryIntervalSeconds" label="Discovery interval (seconds)" min={30} max={3600} step={30} />
              <FormNumberField control={settingsForm.control} name="defaultVolume" label="Default volume (0–1)" min={0} max={1} step={0.05} />
              <FormSwitchField control={settingsForm.control} name="transcodeIncompatible" label="Transcode files the receiver cannot play" className="sm:col-span-2" />
              <FormTextField control={settingsForm.control} name="preferredQuality" label="Preferred quality" placeholder="1080p" />
            </FieldGroup>
          </Panel>
        </SettingsForm>
      ) : null}

      <Dialog isOpen={addingOpen} onOpenChange={setAddingOpen} title="Add cast device" size="sm" footer={<><Button variant="ghost" onPress={() => setAddingOpen(false)}>Cancel</Button><Button variant="primary" onPress={() => void submitDevice()} isPending={adding}>Add</Button></>}>
        <form onSubmit={submitDevice} noValidate>
          <FieldGroup>
            <FormTextField control={deviceForm.control} name="address" label="IP address or hostname" mono isRequired autoFocus />
            <FormNumberField control={deviceForm.control} name="port" label="Port" min={1} max={65535} />
            <FormTextField control={deviceForm.control} name="name" label="Name" description="Optional; the device name is used when empty." />
          </FieldGroup>
        </form>
      </Dialog>
      <ConfirmDialog isOpen={Boolean(removing)} onOpenChange={(open) => !open && setRemoving(null)} title={`Remove ${removing?.name}?`} confirmLabel="Remove" destructive isPending={removingBusy} onConfirm={async () => { if (!removing) return; try { assertSuccess((await removeDevice({ variables: { id: removing.id } })).data?.removeCastDevice, "Could not remove"); setRemoving(null); void devices.refetch(); } catch (error) { toast.danger(errorMessage(error)); } }} />
    </div>
  );
}
