import { useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { toast } from "glass-ux/toast";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconPlus, IconStar, IconTrash } from "glass-ux/icons";
import { IconCast, IconPower, IconRadar } from "@tabler/icons-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { z } from "zod";

import { Button, Dialog, FieldGroup, Panel } from "@/components/ui";
import {
  AddCastDeviceDocument,
  DiscoverCastDevicesDocument,
  EntityCastDeviceListDocument,
  EntityCastDeviceUpdateDocument,
  EntityCastSettingCreateDocument,
  EntityCastSettingListDocument,
  EntityCastSettingUpdateDocument,
  RemoveCastDeviceDocument,
  type CastDeviceFieldsFragment,
} from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { glyph } from "@/lib/entities/kinds";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";


import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsSection } from "./SettingsSection";
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
  const [addDevice] = useMutation(AddCastDeviceDocument);
  const [removeDevice] = useMutation(RemoveCastDeviceDocument);
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
    ...formBehaviour,
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
    ...formBehaviour,
    defaultValues: deviceDefaults,
    validators: { onDynamic: deviceSchema },
    onSubmit: async ({ value }) => {
      const values = deviceSchema.parse(value);
      assertSuccess((await addDevice({ variables: { input: { address: values.address, port: values.port, name: values.name || null } } })).data?.addCastDevice, "Could not add device");
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
  const actions = useMemo(() => defineActions<Device>("cast-device", isAdmin ? [
    {
      id: "favorite", label: (targets) => (targets[0]!.isFavorite ? "Remove favourite" : "Make favourite"), icon: IconStar, group: "organise", priority: 2, visible: (targets) => targets.length === 1,
      run: async (targets) => { const device = targets[0]!; assertSuccess((await updateDevice({ variables: { id: device.id, input: { isFavorite: !device.isFavorite } } })).data?.updateCastDevice, "Could not update"); void refreshDevices(); },
    },
    {
      id: "toggle", label: (targets) => (targets[0]!.enabled === false ? "Enable" : "Disable"), icon: PowerIcon, group: "manage", priority: 1, visible: (targets) => targets.length === 1,
      run: async (targets) => { const device = targets[0]!; assertSuccess((await updateDevice({ variables: { id: device.id, input: { enabled: device.enabled === false } } })).data?.updateCastDevice, "Could not update"); void refreshDevices(); },
    },
    {
      id: "remove", label: "Remove", icon: IconTrash, group: "danger", visible: (targets) => targets.length === 1,
      confirm: (targets) => ({ title: `Remove ${targets[0]!.name}?`, body: targets[0]!.isManual ? "It is forgotten until you add it again." : "Discovery may find it again on the network.", confirmLabel: "Remove", tone: "danger" }),
      run: async (targets) => { assertSuccess((await removeDevice({ variables: { id: targets[0]!.id } })).data?.removeCastDevice, "Could not remove"); void refreshDevices(); },
      successMessage: () => "Device removed",
    },
  ] : []), [isAdmin, refreshDevices, removeDevice, updateDevice]);

  return (
    <div className="flex flex-col gap-6">
      <SettingsSection title="Cast devices" description="Chromecast and Google TV receivers on your network. DLNA renderers are listed for information only." actions={<><Button size="sm" variant="ghost" onPress={() => void runDiscovery()} isPending={discovering}><IconRadar size={16} /> Discover</Button>{isAdmin ? <Button size="sm" variant="primary" onPress={() => { deviceForm.reset(deviceDefaults); setAddingOpen(true); }}><IconPlus /> Add manually</Button> : null}</>}>
        <SettingsLoadError error={devices.error} retry={devices.refetch} hasRows={rows.length > 0} />
        <DataTable<Device> id={`librarian.${user?.id ?? "session"}.settings.cast-devices`} label="Cast devices" columns={columns} rows={rows} getRowId={(device) => device.id} getRowLabel={(device) => device.name} loading={devices.loading && rows.length === 0} actions={actions} noun={{ one: "device", other: "devices" }} emptyMessage="No devices found. Run discovery on the same network as your receivers, or add one by address." />
      </SettingsSection>

      {isAdmin ? (
        <Form form={settingsForm} className="flex min-w-0 flex-col gap-6" onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))}>
          <settingsForm.Subscribe selector={(state) => [state.isSubmitting, state.isDefaultValue]}>
            {([submitting, unchanged]) => <>
              <Panel title="Casting behaviour">
                <FieldGroup columns={2}>
                  <settingsForm.AppField name="autoDiscoveryEnabled">{(field) => <field.SwitchRow label="Discover devices automatically" className="sm:col-span-2" disabled={settings.loading || submitting} />}</settingsForm.AppField>
                  <settingsForm.AppField name="discoveryIntervalSeconds">{(field) => <field.NumberField label="Discovery interval (seconds)" min={30} max={3600} step={30} disabled={settings.loading || submitting} />}</settingsForm.AppField>
                  <settingsForm.AppField name="defaultVolume">{(field) => <field.NumberField label="Default volume (0–1)" min={0} max={1} step={0.05} disabled={settings.loading || submitting} />}</settingsForm.AppField>
                  <settingsForm.AppField name="transcodeIncompatible">{(field) => <field.SwitchRow label="Transcode files the receiver cannot play" className="sm:col-span-2" disabled={settings.loading || submitting} />}</settingsForm.AppField>
                  <settingsForm.AppField name="preferredQuality">{(field) => <field.TextField label="Preferred quality" placeholder="1080p" disabled={settings.loading || submitting} />}</settingsForm.AppField>
                </FieldGroup>
              </Panel>
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
                <deviceForm.AppField name="port">{(field) => <field.NumberField label="Port" min={1} max={65535} disabled={submitting} />}</deviceForm.AppField>
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
    </div>
  );
}
