import { useMutation } from "@apollo/client/react";
import { EntitySettingsDialog, type EntitySettingsTab } from "glass-ux/entity-settings";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { IconInfo, IconSettings, IconTrash } from "glass-ux/icons";
import { SettingRow } from "glass-ux/setting-row";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { z } from "zod";

import { Button, ConfirmDialog } from "@/components/ui";
import { EntityCastDeviceDeleteDocument, EntityCastDeviceUpdateDocument } from "@/graphql/generated/graphql";
import { withoutAction } from "@/lib/actions/withoutAction";
import { formatDateTime, formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { discoveryDetails, makeAndModel, originLabel, playbackBlocker, protocolLabel, reportedDisplayName, type CastDevice, type CastDiscoveryDetails } from "./castDevice";
import { castDeviceIdentityProps } from "./CastDeviceIdentity";
import { useCastDeviceActions, type CastDeviceSettingsTab } from "./useCastDeviceActions";

const schema = z.object({
  name: z.string().trim().min(1, "Give the device a name").max(80, "Use at most 80 characters"),
  enabled: z.boolean(),
  isFavorite: z.boolean(),
});
type Values = z.input<typeof schema>;

const valuesOf = (device: CastDevice): Values => ({ name: device.name, enabled: device.enabled !== false, isFavorite: device.isFavorite });

/** A fact discovery or the receiver reported, read-only. */
function Fact({ label, value, mono, description }: { label: string; value: ReactNode; mono?: boolean; description?: string }) {
  if (value === null || value === undefined || value === "") return null;
  return <SettingRow label={label} description={description} control={<span className={mono ? "font-mono text-body-sm [overflow-wrap:anywhere] text-end" : "text-body-sm text-end"}>{value}</span>} />;
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col" aria-label={title}>
      <h3 className="pb-1 pt-4 text-label-sm font-medium text-muted first:pt-0">{title}</h3>
      {children}
    </section>
  );
}

/** The raw TXT record, key by key. */
function TxtRecord({ txt }: { txt: Record<string, string> }) {
  const keys = Object.keys(txt).sort();
  return (
    <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1 rounded-control bg-surface-secondary p-3 font-mono text-label-sm">
      {keys.map((key) => (
        <div key={key} className="contents">
          <dt className="text-muted">{key}</dt>
          <dd className="[overflow-wrap:anywhere]">{txt[key] || "—"}</dd>
        </div>
      ))}
    </dl>
  );
}

/** "How it was found": everything discovery and the receiver said, grouped by where it came from. */
export function CastDeviceDiscovery({ device }: { device: CastDevice }) {
  const details: CastDiscoveryDetails = discoveryDetails(device);
  const blocker = playbackBlocker(device);
  const capabilities = details.capabilities?.join(", ");
  return (
    <div className="flex flex-col">
      <Group title="Summary">
        <Fact label="Protocol" value={protocolLabel(device)} />
        <Fact label="Added" value={originLabel(device)} />
        <Fact label="Plays from Librarian" value={blocker ? "No" : "Yes"} description={blocker} />
        <Fact label="Address" value={`${device.address}:${device.port}`} mono />
        <Fact label="Maker and model" value={makeAndModel(device)} />
        <Fact label="Model description" value={details.modelDescription} />
        <Fact label="Model number" value={details.modelNumber} />
        <Fact label="Receiver ID" value={device.receiverId} mono />
      </Group>
      <Group title="Discovery">
        <Fact label="First found" value={formatDateTime(device.firstSeenAt ?? device.createdAt)} />
        <Fact label="Last answered" value={device.lastSeenAt ? `${formatRelative(device.lastSeenAt)} · ${formatDateTime(device.lastSeenAt)}` : "Not since it was added"} />
        <Fact label="Last problem" value={device.lastProbeError} />
      </Group>
      {details.protocol === "googleCast" ? (
        <Group title="Google Cast (mDNS)">
          <Fact label="Name it reports" value={details.reportedName} />
          <Fact label="Service" value={details.serviceName} mono />
          <Fact label="Host" value={details.hostname} mono />
          <Fact label="Addresses" value={details.addresses?.join(", ")} mono />
          <Fact label="Can do" value={capabilities} />
          <Fact label="Showing when found" value={details.runningApp} />
          {details.txt && Object.keys(details.txt).length ? (
            <div className="flex flex-col gap-2 pt-3">
              <span className="text-body-sm">TXT record</span>
              <TxtRecord txt={details.txt} />
            </div>
          ) : null}
        </Group>
      ) : null}
      {details.protocol === "dlna" ? (
        <Group title="DLNA (SSDP and UPnP)">
          <Fact label="Name it reports" value={details.reportedName} />
          <Fact label="Description" value={details.descriptionUrl} mono />
          <Fact label="Description problem" value={details.descriptionError} />
          <Fact label="Server" value={details.server} mono />
          <Fact label="USN" value={details.usn} mono />
          <Fact label="Services" value={details.services?.join(", ")} />
        </Group>
      ) : null}
      {!details.protocol && !device.isManual ? (
        <p className="pt-4 text-body-sm text-muted">Nothing more was recorded. The next discovery fills this in.</p>
      ) : null}
    </div>
  );
}

/**
 * A cast receiver's settings: its name, whether it is used and whether it is a favourite (General,
 * administrators), how it was found (anyone), and removing it. The identity and its actions sit in
 * the header.
 */
export function CastDeviceSettingsDialog({ device, isAdmin, open, onOpenChange, tab, onTabChange, onChanged }: {
  device: CastDevice;
  isAdmin: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: CastDeviceSettingsTab;
  onTabChange: (tab: CastDeviceSettingsTab) => void;
  onChanged?: () => void;
}) {
  const [update] = useMutation(EntityCastDeviceUpdateDocument);
  const reported = reportedDisplayName(device);
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: valuesOf(device),
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const values = schema.parse(value);
      // A name that differs from the one the receiver reports is kept through discovery.
      const nameIsCustom = reported ? values.name !== reported : values.name !== device.name || device.nameIsCustom === true;
      assertSuccess((await update({ variables: { id: device.id, input: { ...values, nameIsCustom } } })).data?.updateCastDevice, `${device.name} could not be saved`);
      formApi.reset(values);
      onChanged?.();
    },
  });
  useEffect(() => { if (open) form.reset(valuesOf(device)); }, [open, device.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const actions = useCastDeviceActions<CastDevice>({ isAdmin, open: (_, next) => onTabChange(next ?? "general"), onChanged });
  const headerActions = useMemo(() => withoutAction(actions, "settings", "rename", "details", "remove"), [actions]);
  const [remove, { loading: removing }] = useMutation(EntityCastDeviceDeleteDocument);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const discovered = !(device.isManual && device.discoveryOrigin !== "NETWORK");
  const onRemove = async () => {
    try {
      assertSuccess((await remove({ variables: { id: device.id } })).data?.deleteCastDevice, `${device.name} could not be removed`);
      toast.success(`${device.name} removed`);
      setConfirmRemove(false);
      onOpenChange(false);
      onChanged?.();
    } catch (error) {
      setConfirmRemove(false);
      toast.danger(errorMessage(error));
    }
  };

  const tabs: EntitySettingsTab[] = [
    {
      value: "general", label: "General", icon: <IconSettings />, fields: ["name", "enabled", "isFavorite"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="name">{(field) => (
            <field.TextField
              label="Name"
              required
              disabled={!isAdmin}
              hint={reported ? (device.nameIsCustom ? `Renamed. It calls itself “${reported}”.` : "The name it reports. Rename it to keep your own.") : undefined}
            />
          )}</form.AppField>
          {isAdmin && reported && device.nameIsCustom ? (
            <form.Subscribe selector={(state) => state.values.name}>
              {(name) => (name === reported ? null : (
                <Button size="sm" variant="ghost" className="self-start" onPress={() => form.setFieldValue("name", reported)}>Use “{reported}”</Button>
              ))}
            </form.Subscribe>
          ) : null}
          <form.AppField name="enabled">{(field) => <field.SwitchRow label="Use for casting" description="Turned off, it stays listed but can't be chosen." disabled={!isAdmin} />}</form.AppField>
          <form.AppField name="isFavorite">{(field) => <field.SwitchRow label="Favourite" description="Listed first when you cast." disabled={!isAdmin} />}</form.AppField>
        </div>
      ),
    },
    { value: "discovery", label: "How it was found", icon: <IconInfo />, content: <CastDeviceDiscovery device={device} /> },
    ...(isAdmin ? [{
      value: "remove", label: "Remove", icon: <IconTrash />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">
            {discovered
              ? "Removing it stops anything casting to it. While it is on the network, discovery finds it again, without its name or favourite."
              : "Removing it stops anything casting to it. It is forgotten until you add it again."}
          </p>
          <Button variant="danger-soft" onPress={() => setConfirmRemove(true)}>
            <IconTrash size="md" /> Remove device
          </Button>
        </div>
      ),
    } satisfies EntitySettingsTab] : []),
  ];

  return (
    <>
      <EntitySettingsDialog
        open={open}
        onOpenChange={onOpenChange}
        form={form}
        identity={castDeviceIdentityProps(device, { actions: headerActions })}
        tabs={tabs}
        tab={tab}
        onTabChange={(next) => onTabChange(next as CastDeviceSettingsTab)}
        saveDisabledReason={isAdmin ? undefined : "Only an administrator can change a cast device"}
        onSaveError={(error) => toast.danger(errorMessage(error, "Could not save the device"))}
      />
      <ConfirmDialog
        isOpen={confirmRemove}
        onOpenChange={setConfirmRemove}
        title={`Remove ${device.name}?`}
        description={discovered ? "Discovery may find it again while it is on the network." : "It is forgotten until you add it again."}
        confirmLabel="Remove"
        destructive
        isPending={removing}
        onConfirm={onRemove}
      />
    </>
  );
}
