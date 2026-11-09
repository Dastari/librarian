import { useMutation } from "@apollo/client/react";
import { EntitySettingsDialog, type EntitySettingsTab } from "glass-ux/entity-settings";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { IconInfo, IconSettings, IconTrash } from "glass-ux/icons";
import { SettingRow } from "glass-ux/setting-row";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";
import { z } from "zod";

import { Button, ConfirmDialog } from "@/components/ui";
import { EntityDeviceDeleteDocument, EntityDeviceUpdateDocument, type DeviceFieldsFragment } from "@/graphql/generated/graphql";
import { withoutAction } from "@/lib/actions/withoutAction";
import { formatDateTime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { currentDevice } from "./clientDevice";
import { deviceIdentityProps, deviceSoftware } from "./DeviceIdentity";
import { useDeviceActions, type DeviceSettingsTab } from "./useDeviceActions";

const schema = z.object({ name: z.string().trim().min(1, "Give the device a name").max(80, "Use at most 80 characters") });
type Values = z.input<typeof schema>;

const FORM_FACTORS: Record<string, string> = { tv: "TV", phone: "Phone", tablet: "Tablet", desktop: "Computer", unknown: "Unknown" };

/** A fact the device reported, read-only. */
function Fact({ label, value, mono }: { label: string; value: string | null | undefined; mono?: boolean }) {
  return <SettingRow label={label} control={<span className={mono ? "font-mono text-body-sm [overflow-wrap:anywhere]" : "text-body-sm"}>{value || "Not reported"}</span>} />;
}

/**
 * A device's settings: its name (the one thing that can be changed), what it reported about itself,
 * and signing it out or forgetting it. The identity and its actions sit in the header.
 */
export function DeviceSettingsDialog({ device, owner, open, onOpenChange, tab, onTabChange, onChanged }: {
  device: DeviceFieldsFragment;
  owner?: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tab: DeviceSettingsTab;
  onTabChange: (tab: DeviceSettingsTab) => void;
  onChanged?: () => void;
}) {
  const [update] = useMutation(EntityDeviceUpdateDocument);
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: { name: device.name } as Values,
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const { name } = schema.parse(value);
      assertSuccess((await update({ variables: { id: device.id, input: { name } } })).data?.updateDevice, "The device could not be renamed");
      formApi.reset({ name });
      onChanged?.();
    },
  });
  useEffect(() => { if (open) form.reset({ name: device.name }); }, [open, device.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const actions = useDeviceActions<DeviceFieldsFragment>({ settings: (_, next) => onTabChange(next ?? "general"), onChanged });
  const headerActions = useMemo(() => withoutAction(actions, "settings", "rename", "forget"), [actions]);
  const current = device.id === currentDevice.get();
  const [forget, { loading: forgetting }] = useMutation(EntityDeviceDeleteDocument);
  const [confirmForget, setConfirmForget] = useState(false);
  const onForget = async () => {
    try {
      assertSuccess((await forget({ variables: { id: device.id } })).data?.deleteDevice, `${device.name} could not be forgotten`);
      toast.success(`${device.name} forgotten`);
      setConfirmForget(false);
      onOpenChange(false);
      onChanged?.();
    } catch (error) {
      setConfirmForget(false);
      toast.danger(errorMessage(error));
    }
  };

  const tabs: EntitySettingsTab[] = [
    {
      value: "general", label: "General", icon: <IconSettings />, fields: ["name"],
      content: (
        <div className="flex flex-col gap-3">
          <form.AppField name="name">{(field) => <field.TextField label="Name" required hint={device.nameIsCustom ? "Renamed: the device keeps this name." : "Suggested by the device until you rename it."} />}</form.AppField>
        </div>
      ),
    },
    {
      value: "details", label: "Details", icon: <IconInfo />,
      content: (
        <div className="flex flex-col">
          {owner ? <Fact label="Belongs to" value={owner} /> : null}
          <Fact label="Kind" value={FORM_FACTORS[device.formFactor] ?? device.formFactor} />
          <Fact label="Browser and platform" value={deviceSoftware(device)} />
          <Fact label="App version" value={device.appVersion} />
          <Fact label="Last address" value={device.lastIp} mono />
          <Fact label="Last seen" value={formatDateTime(device.lastSeenAt)} />
          <Fact label="First seen" value={formatDateTime(device.createdAt)} />
          {device.signedOutAt ? <Fact label="Signed out" value={formatDateTime(device.signedOutAt)} /> : null}
          <Fact label="User agent" value={device.userAgent} mono />
        </div>
      ),
    },
    {
      value: "remove", label: "Forget", icon: <IconTrash />,
      content: (
        <div className="flex flex-col items-start gap-3">
          <p className="text-body-sm text-muted">
            {current
              ? "This is the device you are using. Forgetting it signs you out here."
              : "Forgetting a device signs it out and removes it from the list. Its plays stay in the history under its name; it comes back if it signs in again."}
          </p>
          <Button variant="danger-soft" onPress={() => setConfirmForget(true)}>
            <IconTrash size="md" /> Forget device
          </Button>
        </div>
      ),
    },
  ];

  return (
    <>
    <EntitySettingsDialog
      open={open}
      onOpenChange={onOpenChange}
      form={form}
      identity={deviceIdentityProps(device, { actions: headerActions, current, owner })}
      tabs={tabs}
      tab={tab}
      onTabChange={(next) => onTabChange(next as DeviceSettingsTab)}
      onSaveError={(error) => toast.danger(errorMessage(error, "Could not rename the device"))}
    />
    <ConfirmDialog
      isOpen={confirmForget}
      onOpenChange={setConfirmForget}
      title={`Forget ${device.name}?`}
      description="It is signed out and removed from the list. Its plays stay in the history."
      confirmLabel="Forget device"
      destructive
      isPending={forgetting}
      onConfirm={onForget}
    />
    </>
  );
}
