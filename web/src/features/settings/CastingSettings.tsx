import { useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { toast } from "glass-ux/toast";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { IconCast, IconPlus, IconRadar, IconStar, IconStarFilled, IconTrash } from "@tabler/icons-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { z } from "zod";

import { Button, ConfirmDialog, DataTable, type DataTableColumn, type DataTableRowAction, Dialog, EmptyState, FieldGroup, Panel, StatusChip } from "@/components/ui";
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
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";


import { SettingsLoadError } from "./SettingsLoadError";
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

export function CastingSettings() {
  const isAdmin = useIsAdmin();
  const devices = useSettingsConnection(EntityCastDeviceListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.castDevices);
  const rows = devices.rows;
  const settings = useQuery(EntityCastSettingListDocument, { variables: { page: { limit: 1, offset: 0 } } });
  const setting = settings.data?.castSettings.edges[0]?.node;

  const [discover, { loading: discovering }] = useMutation(DiscoverCastDevicesDocument);
  const [addDevice] = useMutation(AddCastDeviceDocument);
  const [removeDevice, { loading: removingBusy }] = useMutation(RemoveCastDeviceDocument);
  const [updateDevice] = useMutation(EntityCastDeviceUpdateDocument);
  const [createSetting] = useMutation(EntityCastSettingCreateDocument);
  const [updateSetting] = useMutation(EntityCastSettingUpdateDocument);
  const [addingOpen, setAddingOpen] = useState(false);
  const [removing, setRemoving] = useState<CastDeviceFieldsFragment | null>(null);

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

  const columns = useMemo<Array<DataTableColumn<CastDeviceFieldsFragment>>>(() => [
    { id: "name", header: "Device", cell: (device) => (<span className="flex items-center gap-2">{device.isFavorite ? <IconStarFilled size={14} className="text-brand" /> : null}<span className="min-w-0"><span className="block truncate text-body-sm text-foreground">{device.name}</span><span className="block truncate font-mono text-label-sm text-muted">{device.address}:{device.port}{device.model ? ` · ${device.model}` : ""}</span></span></span>) },
    { id: "type", header: "Type", size: 130, cell: (device) => <span className="text-muted">{device.deviceType}</span> },
    { id: "playback", header: "Playback", size: 150, cell: (device) => (device.playbackSupported ? <StatusChip status={{ label: "Supported", tone: "success", dot: "bg-success" }} /> : <StatusChip status={{ label: "Discovery only", tone: "default", dot: "bg-muted" }} />) },
    { id: "seen", header: "Seen", size: 140, hideBelow: "md", cell: (device) => <span className="text-muted">{device.lastSeenAt ? formatRelative(device.lastSeenAt) : device.isManual ? "Manual" : "—"}</span> },
    { id: "probe", header: "Last probe", size: 200, hideBelow: "lg", cell: (device) => <span className={device.lastProbeError ? "text-danger" : "text-muted"}>{device.lastProbeError ?? (device.lastProbeAt ? formatRelative(device.lastProbeAt) : "—")}</span> },
  ], []);
  const actions: Array<DataTableRowAction<CastDeviceFieldsFragment>> = isAdmin
    ? [
        { key: "favorite", label: (device) => (device.isFavorite ? "Remove favourite" : "Favourite"), icon: <IconStar size={16} />, onAction: async (device) => { try { assertSuccess((await updateDevice({ variables: { id: device.id, input: { isFavorite: !device.isFavorite } } })).data?.updateCastDevice, "Could not update"); void devices.refetch(); } catch (error) { toast.danger(errorMessage(error)); } } },
        { key: "toggle", label: (device) => (device.enabled === false ? "Enable" : "Disable"), onAction: async (device) => { try { assertSuccess((await updateDevice({ variables: { id: device.id, input: { enabled: device.enabled === false } } })).data?.updateCastDevice, "Could not update"); void devices.refetch(); } catch (error) { toast.danger(errorMessage(error)); } } },
        { key: "remove", label: "Remove", icon: <IconTrash size={16} />, destructive: true, onAction: (device) => setRemoving(device) },
      ]
    : [];

  return (
    <div className="flex flex-col gap-6">
      <Panel title="Cast devices" description="Chromecast and Google TV receivers on your network. DLNA renderers are listed for information only." flush actions={<><Button size="sm" variant="ghost" onPress={() => void runDiscovery()} isPending={discovering}><IconRadar size={16} /> Discover</Button>{isAdmin ? <Button size="sm" variant="primary" onPress={() => { deviceForm.reset(deviceDefaults); setAddingOpen(true); }}><IconPlus size={16} /> Add manually</Button> : null}</>}>
        <SettingsLoadError error={devices.error} retry={devices.refetch} hasRows={rows.length > 0} />
        <DataTable<CastDeviceFieldsFragment> className="px-4 pb-4" frame={false} columns={columns} rows={rows} getRowId={(device) => device.id} isLoading={devices.loading && rows.length === 0} rowActions={actions} noun="devices" emptyState={<EmptyState compact icon={IconCast} title="No devices found" description="Run discovery on the same network as your receivers, or add one by address." />} />
      </Panel>

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
      <ConfirmDialog isOpen={Boolean(removing)} onOpenChange={(open) => !open && setRemoving(null)} title={`Remove ${removing?.name}?`} confirmLabel="Remove" destructive isPending={removingBusy} onConfirm={async () => { if (!removing) return; try { assertSuccess((await removeDevice({ variables: { id: removing.id } })).data?.removeCastDevice, "Could not remove"); setRemoving(null); void devices.refetch(); } catch (error) { toast.danger(errorMessage(error)); } }} />
    </div>
  );
}
