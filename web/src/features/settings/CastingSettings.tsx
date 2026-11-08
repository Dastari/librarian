import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { copyText } from "glass-ux/copy-button";
import { useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { toast } from "glass-ux/toast";
import { Form, FormActions, useAppForm } from "glass-ux/form";
import { defineActions, type ActionDefinition } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, IdentityCell, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconCopy, IconPlus, IconStar, IconTrash } from "glass-ux/icons";
import { IconCast, IconPower, IconRadar } from "@tabler/icons-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { z } from "zod";

import { Button, Dialog, FieldGroup } from "@/components/ui";
import {
  DiscoverCastDevicesDocument,
  EntityCastDeviceCreateDocument,
  EntityCastDeviceDeleteDocument,
  EntityCastDeviceListDocument,
  EntityCastDeviceUpdateDocument,
  EntityCastSettingCreateDocument,
  EntityCastSettingListDocument,
  EntityCastSettingUpdateDocument,
  type CastDeviceFieldsFragment,
} from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { glyph } from "@/lib/entities/kinds";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";


import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { runSettingsSelection } from "./runSettingsSelection";

import { SettingsLoadError } from "./SettingsLoadError";
import { DurationRow, PercentRow, TextRow, settingsFormBehaviour } from "./SettingFields";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

const requiredNumber = (rule: z.ZodNumber) => z.number().nullable().transform((value, context) => {
  if (value === null) {
    context.addIssue({ code: "custom", message: "Enter a number" });
    return z.NEVER;
  }
  return value;
}).pipe(rule);

const settingsSchema = z.object({ autoDiscoveryEnabled: z.boolean(), discoveryIntervalSeconds: requiredNumber(z.number().int().min(30).max(3600)), defaultVolume: requiredNumber(z.number().min(0).max(1)), transcodeIncompatible: z.boolean(), preferredQuality: z.string().trim() });
type SettingsValues = z.input<typeof settingsSchema>;
const deviceSchema = z.object({ name: z.string().trim(), address: z.string().trim().min(1, "Enter the device address"), port: requiredNumber(z.number().int().min(1).max(65535)) });
type DeviceValues = z.input<typeof deviceSchema>;
type Device = CastDeviceFieldsFragment;

const CastGlyph = glyph(IconCast, "cast");
const PowerIcon = glyph(IconPower, "power");
const column = createDataTableColumns<Device>();
const columns = column.columns([
  column.accessor("name", {
    header: "Device", width: 24, priority: 1, cardRole: "title", sortable: true,
    cell: ({ row }) => { const device = row.original; return <IdentityCell icon={<CastGlyph />} title={`${device.isFavorite ? "★ " : ""}${device.name}`} subtitle={`${device.address}:${device.port}${device.model ? ` · ${device.model}` : ""}`} status={device.enabled === false ? "offline" : device.lastProbeError ? "error" : "online"} statusLabel={device.enabled === false ? "Disabled" : device.lastProbeError ? "Probe failed" : "Available"} />; },
  }),
  column.accessor("deviceType", { header: "Type", width: 9, priority: 3, sortable: true, cell: ({ row }) => <TextCell muted>{row.original.deviceType}</TextCell> }),
  column.display({ id: "playback", header: "Playback", width: 9, priority: 2, cardRole: "status", cell: ({ row }) => (row.original.playbackSupported ? <Badge size="sm" marker="dot" tone="success">Supported</Badge> : <Badge size="sm" tone="neutral">Discovery only</Badge>) }),
  column.accessor((device) => device.lastSeenAt ?? "", { id: "seen", header: "Seen", width: 8, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastSeenAt)} fallback={row.original.isManual ? "Manual" : "—"} /> }),
  column.display({ id: "probe", header: "Last probe", width: 14, priority: 4, cell: ({ row }) => (row.original.lastProbeError ? <TextCell><span className="text-danger">{row.original.lastProbeError}</span></TextCell> : <RelativeTime value={parseTimestamp(row.original.lastProbeAt)} />) }),
]);

export function CastingSettings() {
  const isAdmin = useIsAdmin();
  const devices = useSettingsConnection(EntityCastDeviceListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.castDevices);
  const rows = devices.rows;
  const settings = useQuery(EntityCastSettingListDocument, { variables: { page: { limit: 1, offset: 0 } } });
  const setting = settings.data?.castSettings.edges[0]?.node;

  const [discover, { loading: discovering }] = useMutation(DiscoverCastDevicesDocument);
  const [addDevice] = useMutation(EntityCastDeviceCreateDocument);
  const [removeDevice] = useMutation(EntityCastDeviceDeleteDocument);
  const { user } = useSession();
  const [updateDevice] = useMutation(EntityCastDeviceUpdateDocument);
  const [createSetting] = useMutation(EntityCastSettingCreateDocument);
  const [updateSetting] = useMutation(EntityCastSettingUpdateDocument);
  const [addingOpen, setAddingOpen] = useState(false);

  const storedValues = useMemo<SettingsValues>(() => ({
    autoDiscoveryEnabled: setting?.autoDiscoveryEnabled ?? true,
    discoveryIntervalSeconds: setting?.discoveryIntervalSeconds ?? 300,
    defaultVolume: setting?.defaultVolume ?? 0.8,
    transcodeIncompatible: setting?.transcodeIncompatible ?? true,
    preferredQuality: setting?.preferredQuality ?? "",
  }), [setting]);
  const defaults = useRef(storedValues);
  const settingsForm = useAppForm({
    ...settingsFormBehaviour,
    defaultValues: defaults.current,
    validators: { onDynamic: settingsSchema },
    onSubmit: async ({ value, formApi }) => {
      const values = settingsSchema.parse(value);
      const input = { ...values, preferredQuality: values.preferredQuality || null };
      if (setting) assertSuccess((await updateSetting({ variables: { id: setting.id, input } })).data?.updateCastSetting, "Could not save");
      else assertSuccess((await createSetting({ variables: { input } })).data?.createCastSetting, "Could not save");
      defaults.current = values;
      formApi.reset(values);
      toast.success("Settings saved");
      void settings.refetch();
    },
  });
  useEffect(() => {
    if (settings.loading || settingsForm.state.isSubmitting) return;
    const current = settingsForm.state.values;
    const edited = (Object.keys(storedValues) as Array<keyof SettingsValues>).filter((key) => !Object.is(current[key], defaults.current[key]));
    defaults.current = storedValues;
    settingsForm.reset(storedValues);
    for (const key of edited) settingsForm.setFieldValue(key, current[key]);
  }, [storedValues, settings.loading, settingsForm]);
  const deviceDefaults: DeviceValues = { name: "", address: "", port: 8009 };
  const deviceForm = useAppForm({
    ...settingsFormBehaviour,
    defaultValues: deviceDefaults,
    validators: { onDynamic: deviceSchema },
    onSubmit: async ({ value }) => {
      const values = deviceSchema.parse(value);
      // A blank name is defaulted from the address by the server's write transform.
      assertSuccess((await addDevice({ variables: { input: { name: values.name, address: values.address, port: values.port, deviceType: "CHROMECAST", isFavorite: false, isManual: true, enabled: true, discoveryOrigin: "MANUAL", playbackSupported: true, firstSeenAt: new Date().toISOString() } } })).data?.createCastDevice, "Could not add device");
      toast.success("Device added");
      setAddingOpen(false);
      void devices.refetch();
    },
  });

  const runDiscovery = async () => {
    try {
      const { data } = await discover();
      toast.success(`Found ${data?.discoverCastDevices.length ?? 0} devices`);
      void devices.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const refreshDevices = devices.refetch;
  const actions = useMemo(() => defineActions<Device>("cast-device", [
    { id: "copy-address", label: (targets) => targets.length === 1 ? "Copy receiver address" : "Copy receiver addresses", icon: IconCopy, group: "open", bulk: true,
      run: (targets) => copyText(targets.map((device) => `${device.name}: ${device.address}:${device.port}`).join("\n")), successMessage: () => "Receiver addresses copied",
    },
    ...(isAdmin ? ([
    {
      id: "favorite", bulk: true, label: (targets) => (targets[0]!.isFavorite ? "Remove favourite" : "Make favourite"), icon: IconStar, group: "organise", priority: 2, visible: (targets) => targets.length === 1,
      run: async (targets) => { const device = targets[0]!; assertSuccess((await updateDevice({ variables: { id: device.id, input: { isFavorite: !device.isFavorite } } })).data?.updateCastDevice, "Could not update"); void refreshDevices(); },
    },
    {
      id: "toggle", bulk: true, label: (targets) => (targets[0]!.enabled === false ? "Enable" : "Disable"), icon: PowerIcon, group: "manage", priority: 1, visible: (targets) => targets.length === 1,
      run: async (targets) => { const device = targets[0]!; assertSuccess((await updateDevice({ variables: { id: device.id, input: { enabled: device.enabled === false } } })).data?.updateCastDevice, "Could not update"); void refreshDevices(); },
    },
    {
      id: "remove", label: (targets: readonly Device[]) => targets.length === 1 ? "Remove" : `Remove ${targets.length} devices`, icon: IconTrash, group: "danger" as const, bulk: true,
      confirm: (targets: readonly Device[]) => ({ title: targets.length === 1 ? `Remove ${targets[0]!.name}?` : `Remove ${targets.length} devices?`, body: targets.every((target) => target.isManual) ? "They are forgotten until you add them again." : "Discovery may find these devices again on the network.", confirmLabel: "Remove", tone: "danger" as const }),
      run: (targets: readonly Device[]) => runSettingsSelection(targets, "Remove", (target) => target.name, async (target) => assertSuccess((await removeDevice({ variables: { id: target.id } })).data?.deleteCastDevice, "Could not remove"), refreshDevices),
      successMessage: (targets: readonly Device[]) => targets.length === 1 ? "Device removed" : `${targets.length} devices removed`,
    },
    ...([true, false] as const).map((enabled) => ({
      id: enabled ? "enable-selected" : "disable-selected", label: enabled ? "Enable devices" : "Disable devices", icon: PowerIcon, group: "manage" as const, bulk: true,
      visible: (targets: readonly Device[]) => targets.length > 1 && targets.some((target) => (target.enabled !== false) !== enabled),
      run: (targets: readonly Device[]) => runSettingsSelection(targets.filter((target) => (target.enabled !== false) !== enabled), enabled ? "Enable" : "Disable", (target) => target.name, async (target) => assertSuccess((await updateDevice({ variables: { id: target.id, input: { enabled } } })).data?.updateCastDevice, "Could not update"), refreshDevices),
      successMessage: () => enabled ? "Devices enabled" : "Devices disabled",
    })),
    ...([true, false] as const).map((isFavorite) => ({
      id: isFavorite ? "favourite-selected" : "unfavourite-selected", label: isFavorite ? "Make favourites" : "Remove favourites", icon: IconStar, group: "organise" as const, bulk: true,
      visible: (targets: readonly Device[]) => targets.length > 1 && targets.some((target) => Boolean(target.isFavorite) !== isFavorite),
      run: (targets: readonly Device[]) => runSettingsSelection(targets.filter((target) => Boolean(target.isFavorite) !== isFavorite), "Update favourites", (target) => target.name, async (target) => assertSuccess((await updateDevice({ variables: { id: target.id, input: { isFavorite } } })).data?.updateCastDevice, "Could not update"), refreshDevices),
      successMessage: () => "Favourites updated",
    })),
  ] satisfies ActionDefinition<Device>[]) : []),
  ]), [isAdmin, refreshDevices, removeDevice, updateDevice]);

  return (
    <SettingsPage>
      <SettingsSection title="Cast devices" description="Chromecast and Google TV receivers on your network. DLNA renderers are listed for information only." actions={<><Button size="sm" variant="ghost" onPress={() => void runDiscovery()} isPending={discovering}><IconRadar size={16} /> Discover</Button>{isAdmin ? <Button size="sm" variant="primary" onPress={() => { deviceForm.reset(deviceDefaults); setAddingOpen(true); }}><IconPlus /> Add manually</Button> : null}</>}>
        <SettingsLoadError error={devices.error} retry={devices.refetch} hasRows={rows.length > 0} />
        <LoadedDataTable<Device> complete={!devices.loading && !devices.error} height="auto" selectable queryKey={user?.id} id={`librarian.${user?.id ?? "session"}.settings.cast-devices`} label="Cast devices" columns={columns} rows={rows} getRowId={(device) => device.id} getRowLabel={(device) => device.name} loading={devices.loading && rows.length === 0} actions={actions} renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={{ one: "device", other: "devices" }} />} noun={{ one: "device", other: "devices" }} emptyMessage="No devices found. Run discovery on the same network as your receivers, or add one by address." />
      </SettingsSection>

      {isAdmin ? (
        <Form form={settingsForm} className="settings-measure flex min-w-0 flex-col gap-4" onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))}>
          <settingsForm.Subscribe selector={(state) => [state.isSubmitting, state.isDefaultValue]}>
            {([submitting, unchanged]) => <>
              <SettingsSection title="Casting behaviour" width="readable">
                <settingsForm.AppField name="autoDiscoveryEnabled">{(field) => <field.SwitchRow label="Discover devices automatically" disabled={settings.loading || submitting} />}</settingsForm.AppField>
                <settingsForm.AppField name="discoveryIntervalSeconds">{() => <DurationRow label="Discovery interval" units={["m", "s"]} defaultUnit="s" disabled={settings.loading || submitting} />}</settingsForm.AppField>
                <settingsForm.AppField name="defaultVolume">{() => <PercentRow label="Default volume" min={0} max={1} step={0.05} disabled={settings.loading || submitting} />}</settingsForm.AppField>
                <settingsForm.AppField name="transcodeIncompatible">{(field) => <field.SwitchRow label="Transcode files the receiver cannot play" disabled={settings.loading || submitting} />}</settingsForm.AppField>
                <settingsForm.AppField name="preferredQuality">{() => <TextRow label="Preferred quality" placeholder="1080p" disabled={settings.loading || submitting} />}</settingsForm.AppField>
              </SettingsSection>
              <FormActions>
                <settingsForm.CancelButton disabled={settings.loading || submitting || unchanged} onCancel={() => { defaults.current = storedValues; settingsForm.reset(storedValues); }}>Discard</settingsForm.CancelButton>
                <settingsForm.SubmitButton disabled={settings.loading || submitting}>Save</settingsForm.SubmitButton>
              </FormActions>
            </>}
          </settingsForm.Subscribe>
        </Form>
      ) : null}

      <Dialog isOpen={addingOpen} onOpenChange={setAddingOpen} title="Add cast device" size="sm">
        <Form form={deviceForm} onError={(error) => toast.danger(errorMessage(error))}>
          <deviceForm.Subscribe selector={(state) => state.isSubmitting}>
            {(submitting) => <>

              <FieldGroup>
                <deviceForm.AppField name="address">{(field) => <field.TextField label="IP address or hostname" className="font-mono" required disabled={submitting} />}</deviceForm.AppField>
                <deviceForm.AppField name="port">{(field) => <field.PortField label="Port" disabled={submitting} />}</deviceForm.AppField>
                <deviceForm.AppField name="name">{(field) => <field.TextField label="Name" hint="Optional; the device name is used when empty." disabled={submitting} />}</deviceForm.AppField>
              </FieldGroup>
              <FormActions>
                <deviceForm.CancelButton disabled={submitting} onCancel={() => setAddingOpen(false)}>Cancel</deviceForm.CancelButton>
                <deviceForm.SubmitButton disabled={submitting}>Add</deviceForm.SubmitButton>
              </FormActions>
            </>}
          </deviceForm.Subscribe>
        </Form>
      </Dialog>
    </SettingsPage>
  );
}
