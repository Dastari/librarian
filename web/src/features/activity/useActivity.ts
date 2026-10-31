import type { DataTableState } from "glass-ux/data-table";
import { useMemo } from "react";

import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import { ActivityCountsDocument, ActivityTableDocument, type ActivityTableQuery, type NotificationOrderByInput, type NotificationWhereInput } from "@/graphql/generated/graphql";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

export type ActivityRow = ActivityTableQuery["notifications"]["edges"][number]["node"];

export const ACTIVITY_PRESETS = ["unread", "action", "all"] as const;
export type ActivityPreset = (typeof ACTIVITY_PRESETS)[number];

/** Categories the server writes (library_scan.rs, quality, auto-download, metadata jobs). */
export const ACTIVITY_CATEGORIES = ["ORGANIZATION", "QUALITY", "SCAN", "auto_download", "metadata", "organize", "filesystem", "llm"] as const;

const BATCH = 50;

function presetWhere(preset: ActivityPreset): NotificationWhereInput | null {
  switch (preset) {
    case "unread": return { readAt: { isNull: true } };
    case "action": return { actionType: { isNull: false }, resolvedAt: { isNull: true } };
    default: return null;
  }
}

function optionValues(raw: unknown): string[] {
  if (raw && typeof raw === "object" && "values" in raw && Array.isArray(raw.values)) return raw.values.filter((value): value is string => typeof value === "string");
  return [];
}

/** The table's state as a server query; only the columns' declared filters cross over. */
export function activityVariables(state: DataTableState, preset: ActivityPreset): { where: NotificationWhereInput; orderBy: NotificationOrderByInput[] } {
  const conditions: NotificationWhereInput[] = [];
  const base = presetWhere(preset);
  if (base) conditions.push(base);
  for (const { id, value } of state.filters) {
    const values = optionValues(value);
    if (!values.length) continue;
    switch (id) {
      case "level": conditions.push({ notificationType: { inList: values } }); break;
      case "category": conditions.push({ category: { inList: values } }); break;
      case "library": conditions.push({ libraryId: { inList: values } }); break;
      default: throw new Error(`Unsupported activity filter: ${id}`);
    }
  }
  const query = state.search?.trim();
  if (query) conditions.push({ or: [{ title: { contains: query } }, { message: { contains: query } }, { category: { contains: query } }] });

  const orderBy = state.sorting.flatMap(({ id, desc }): NotificationOrderByInput[] => {
    const direction = desc ? "DESC" : "ASC";
    switch (id) {
      case "when": return [{ createdAt: direction }];
      case "title": return [{ title: direction }];
      case "level": return [{ notificationType: direction }];
      case "category": return [{ category: direction }];
      default: return [];
    }
  });
  return { where: conditions.length ? { and: conditions } : {}, orderBy: orderBy.length ? orderBy : [{ createdAt: "DESC" }] };
}

/** Notifications in server batches, with the preset counts beside them. */
export function useActivity(state: DataTableState, preset: ActivityPreset) {
  const variables = useMemo(() => activityVariables(state, preset), [state, preset]);
  const list = useSettingsConnection(ActivityTableDocument, variables, (data) => data.notifications, { infinite: true, batchSize: BATCH, live: true });
  const counts = useQuery(ActivityCountsDocument);
  const presetCounts: Partial<Record<ActivityPreset, number>> = counts.data ? {
    unread: counts.data.unread.pageInfo.totalCount ?? undefined,
    action: counts.data.action.pageInfo.totalCount ?? undefined,
    all: counts.data.all.pageInfo.totalCount ?? undefined,
  } : {};
  const refetch = () => {
    void list.refetch().catch(() => {});
    void counts.refetch();
  };
  return { list, counts: presetCounts, variables, refetch };
}
