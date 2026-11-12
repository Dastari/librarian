import { useMutation } from "@apollo/client/react";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { Form, FormActions, useAppForm } from "glass-ux/form";
import { IconPlus } from "glass-ux/icons";
import { Notice } from "glass-ux/notice";
import { toast } from "glass-ux/toast";
import { IconRadar } from "@tabler/icons-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { z } from "zod";

import { Button, Dialog, FieldGroup } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { castProtocol, playbackBlocker, protocolLabel, type CastDevice } from "@/features/casting/castDevice";
import { CastDeviceIdentity } from "@/features/casting/CastDeviceIdentity";
import { CastDeviceSettingsDialog } from "@/features/casting/CastDeviceSettingsDialog";
import { useCastDeviceActions, type CastDeviceSettingsTab } from "@/features/casting/useCastDeviceActions";
import {
  DiscoverCastDevicesDocument,
  EntityCastDeviceCreateDocument,
  EntityCastDeviceListDocument,
  EntityCastSettingCreateDocument,
  EntityCastSettingListDocument,
  EntityCastSettingUpdateDocument,
} from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { SettingsLoadError } from "./SettingsLoadError";
import { DurationRow, PercentRow, SelectRow, settingsFormBehaviour } from "./SettingFields";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

const requiredNumber = (rule: z.ZodNumber) => z.number().nullable().transform((value, context) => {
  if (value === null) {
    context.addIssue({ code: "custom", message: "Enter a number" });
    return z.NEVER;
  }
  return value;
}).pipe(rule);

const QUALITIES = ["original", "2160p", "1080p", "720p", "480p"] as const;
type Quality = (typeof QUALITIES)[number];
const QUALITY_OPTIONS = [
  { value: "original" as Quality, label: "No limit", description: "Send the original when the receiver can play it" },
  { value: "2160p" as Quality, label: "4K (2160p)" },
  { value: "1080p" as Quality, label: "1080p" },
  { value: "720p" as Quality, label: "720p" },
  { value: "480p" as Quality, label: "480p" },
];

const settingsSchema = z.object({ autoDiscoveryEnabled: z.boolean(), discoveryIntervalSeconds: requiredNumber(z.number().int().min(30).max(3600)), defaultVolume: requiredNumber(z.number().min(0).max(1)), transcodeIncompatible: z.boolean(), preferredQuality: z.enum(QUALITIES) });
type SettingsValues = z.input<typeof settingsSchema>;
const deviceSchema = z.object({ name: z.string().trim(), address: z.string().trim().min(1, "Enter the device address"), port: requiredNumber(z.number().int().min(1).max(65535)) });
type DeviceValues = z.input<typeof deviceSchema>;

const asQuality = (value: string | null | undefined): Quality => (QUALITIES as readonly string[]).includes(value?.trim().toLowerCase() ?? "") ? (value!.trim().toLowerCase() as Quality) : "original";

/** Playable now, listed only, or turned off. */
function PlaysBadge({ device }: { device: CastDevice }) {
  if (device.enabled === false) return <Badge size="sm" tone="neutral">Turned off</Badge>;
  if (!playbackBlocker(device)) return <Badge size="sm" marker="dot" tone="success">Ready</Badge>;
  return <Badge size="sm" tone="neutral">Listed only</Badge>;
}

const column = createDataTableColumns<CastDevice>();
const columns = column.columns([
  column.accessor("name", { header: "Device", width: 30, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => <CastDeviceIdentity device={row.original} /> }),
  column.accessor((device) => protocolLabel(device), { id: "protocol", header: "Protocol", width: 10, priority: 3, sortable: true, cell: ({ row }) => <TextCell muted>{protocolLabel(row.original)}</TextCell> }),
  column.display({ id: "plays", header: "Casting", width: 9, priority: 2, cardRole: "status", cell: ({ row }) => <PlaysBadge device={row.original} /> }),
  column.accessor((device) => device.lastSeenAt ?? "", { id: "seen", header: "Last answered", width: 9, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastSeenAt)} fallback={row.original.isManual ? "Added by address" : "—"} /> }),
]);
const noun = { one: "device", other: "devices" };

/**
 * Settings › Casting: the receivers Librarian knows (found on this server's network or added by
 * address), each opening its own dialog with how it was found, and how casting behaves.
 */
export function CastingSettings() {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const devices = useSettingsConnection(EntityCastDeviceListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.castDevices);
  const { rows, refetch } = devices;
  const refresh = useCallback(() => void refetch(), [refetch]);
  const settings = useQuery(EntityCastSettingListDocument, { variables: { page: { limit: 1, offset: 0 } } });
  const setting = settings.data?.castSettings.edges[0]?.node;

  const [discover, { loading: discovering }] = useMutation(DiscoverCastDevicesDocument);
  const [addDevice] = useMutation(EntityCastDeviceCreateDocument);
  const [createSetting] = useMutation(EntityCastSettingCreateDocument);
  const [updateSetting] = useMutation(EntityCastSettingUpdateDocument);
  const [addingOpen, setAddingOpen] = useState(false);
  const [editing, setEditing] = useState<{ id: string; tab: CastDeviceSettingsTab } | null>(null);
  const openDevice = useCallback((device: CastDevice, tab?: CastDeviceSettingsTab) => setEditing({ id: device.id, tab: tab ?? (isAdmin ? "general" : "discovery") }), [isAdmin]);
  const actions = useCastDeviceActions<CastDevice>({ isAdmin, open: openDevice, onChanged: refresh });
  const editingRow = editing ? rows.find((row) => row.id === editing.id) ?? null : null;
  const listedOnly = rows.filter((row) => castProtocol(row) === "dlna").length;
  const playable = rows.filter((row) => !playbackBlocker(row)).length;

  const storedValues = useMemo<SettingsValues>(() => ({
    autoDiscoveryEnabled: setting?.autoDiscoveryEnabled ?? true,
    discoveryIntervalSeconds: setting?.discoveryIntervalSeconds ?? 300,
    defaultVolume: setting?.defaultVolume ?? 0.8,
    transcodeIncompatible: setting?.transcodeIncompatible ?? true,
    preferredQuality: asQuality(setting?.preferredQuality),
  }), [setting]);
  const defaults = useRef(storedValues);
  const settingsForm = useAppForm({
    ...settingsFormBehaviour,
    defaultValues: defaults.current,
    validators: { onDynamic: settingsSchema },
    onSubmit: async ({ value, formApi }) => {
      const values = settingsSchema.parse(value);
      const input = { ...values, preferredQuality: values.preferredQuality === "original" ? null : values.preferredQuality };
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
      assertSuccess((await addDevice({ variables: { input: { name: values.name, address: values.address, port: values.port, deviceType: "CHROMECAST", isFavorite: false, isManual: true, enabled: true, discoveryOrigin: "MANUAL", playbackSupported: true, firstSeenAt: new Date().toISOString(), nameIsCustom: values.name.length > 0 } } })).data?.createCastDevice, "Could not add device");
      toast.success("Device added");
      setAddingOpen(false);
      refresh();
    },
  });

  const runDiscovery = async () => {
    try {
      const { data } = await discover();
      const found = data?.discoverCastDevices.length ?? 0;
      toast.success(found === 1 ? "Found 1 receiver" : `Found ${found} receivers`);
      refresh();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <SettingsPage>
      <SettingsSection
        title="Cast devices"
        description="TVs, speakers and dongles on this server's network. Google Cast receivers play from Librarian; DLNA ones are listed only."
        actions={isAdmin ? <>
          <Button size="sm" variant="ghost" onPress={() => void runDiscovery()} isPending={discovering}><IconRadar size={16} /> Look now</Button>
          <Button size="sm" variant="primary" onPress={() => { deviceForm.reset(deviceDefaults); setAddingOpen(true); }}><IconPlus /> Add by address</Button>
        </> : undefined}
      >
        <SettingsLoadError error={devices.error} retry={refetch} hasRows={rows.length > 0} />
        {!devices.loading && rows.length > 0 && playable === 0 && listedOnly > 0 ? (
          <Notice tone="info">
            {listedOnly === 1 ? "The receiver found so far uses DLNA, which" : "The receivers found so far use DLNA, which"} Librarian can list but not play to yet. A Chromecast, a Google TV or a TV with Chromecast built in on the same network appears here and can play.
          </Notice>
        ) : null}
        <LoadedDataTable<CastDevice>
          complete={!devices.loading && !devices.error}
          height="auto"
          selectable={isAdmin}
          queryKey={user?.id}
          id={`librarian.${user?.id ?? "session"}.settings.cast-devices`}
          label="Cast devices"
          columns={columns}
          rows={rows}
          getRowId={(device) => device.id}
          getRowLabel={(device) => device.name}
          loading={devices.loading && rows.length === 0}
          actions={actions}
          renderBulkActions={(selected) => <TableBulkActions set={actions} rows={selected} noun={noun} />}
          onRowOpen={(row) => openDevice(row)}
          rowOpen="click"
          noun={noun}
          emptyMessage="No receivers yet. Discovery only sees this server's own network: add one by its address if it is elsewhere."
        />
      </SettingsSection>

      {isAdmin ? (
        <Form form={settingsForm} className="settings-measure flex min-w-0 flex-col gap-4" onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))}>
          <settingsForm.Subscribe selector={(state) => [state.isSubmitting, state.isDefaultValue]}>
            {([submitting, unchanged]) => <>
              <SettingsSection title="Discovery" width="readable">
                <settingsForm.AppField name="autoDiscoveryEnabled">{(field) => <field.SwitchRow label="Look for receivers automatically" description="Google Cast (mDNS) and DLNA (SSDP) on this server's network." disabled={settings.loading || submitting} />}</settingsForm.AppField>
                <settingsForm.AppField name="discoveryIntervalSeconds">{() => <DurationRow label="Look every" units={["m", "s"]} defaultUnit="m" disabled={settings.loading || submitting} />}</settingsForm.AppField>
              </SettingsSection>
              <SettingsSection title="Playback" width="readable">
                <settingsForm.AppField name="preferredQuality">{() => <SelectRow<Quality> label="Highest quality" description="Anything above it is converted down first." options={QUALITY_OPTIONS} disabled={settings.loading || submitting} />}</settingsForm.AppField>
                <settingsForm.AppField name="transcodeIncompatible">{(field) => <field.SwitchRow label="Convert what the receiver can't play" description="Otherwise those files can't be cast." disabled={settings.loading || submitting} />}</settingsForm.AppField>
                <settingsForm.AppField name="defaultVolume">{() => <PercentRow label="Starting volume" min={0} max={1} step={0.05} disabled={settings.loading || submitting} />}</settingsForm.AppField>
              </SettingsSection>
              <FormActions>
                <settingsForm.CancelButton disabled={settings.loading || submitting || unchanged} onCancel={() => { defaults.current = storedValues; settingsForm.reset(storedValues); }}>Discard</settingsForm.CancelButton>
                <settingsForm.SubmitButton disabled={settings.loading || submitting}>Save</settingsForm.SubmitButton>
              </FormActions>
            </>}
          </settingsForm.Subscribe>
        </Form>
      ) : null}

      {editingRow ? (
        <CastDeviceSettingsDialog
          device={editingRow}
          isAdmin={isAdmin}
          open={editing !== null}
          onOpenChange={(next) => !next && setEditing(null)}
          tab={editing?.tab ?? "general"}
          onTabChange={(tab) => setEditing((state) => (state ? { ...state, tab } : state))}
          onChanged={refresh}
        />
      ) : null}

      <Dialog isOpen={addingOpen} onOpenChange={setAddingOpen} title="Add a receiver by address" size="sm">
        <Form form={deviceForm} onError={(error) => toast.danger(errorMessage(error))}>
          <deviceForm.Subscribe selector={(state) => state.isSubmitting}>
            {(submitting) => <>
              <p className="pb-3 text-body-sm text-muted">For a Google Cast receiver discovery can't see, such as one on another subnet. It must still be reachable from this server.</p>
              <FieldGroup>
                <deviceForm.AppField name="address">{(field) => <field.TextField label="IP address" className="font-mono" required disabled={submitting} />}</deviceForm.AppField>
                <deviceForm.AppField name="port">{(field) => <field.PortField label="Port" hint="8009 for Google Cast." disabled={submitting} />}</deviceForm.AppField>
                <deviceForm.AppField name="name">{(field) => <field.TextField label="Name" hint="Optional." disabled={submitting} />}</deviceForm.AppField>
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
