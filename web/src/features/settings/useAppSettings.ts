import { useApolloClient, useQuery } from "@apollo/client/react";
import { useCallback, useMemo } from "react";

import { AppSettingsByCategoryDocument, EntityAppSettingCreateDocument, EntityAppSettingUpdateDocument } from "@/graphql/generated/graphql";

/**
 * App settings are key/value rows grouped by category. Values are stored as strings; JSON
 * strings and quoted values are normalised on read so forms see plain values.
 */
export function useAppSettings(category: string) {
  const client = useApolloClient();
  const { data, previousData, loading, error, refetch } = useQuery(AppSettingsByCategoryDocument, { variables: { category } });
  const rows = useMemo(() => (data ?? previousData)?.appSettings.edges.map((edge) => edge.node) ?? [], [data, previousData]);
  const values = useMemo(() => {
    const map = new Map<string, string>();
    for (const row of rows) map.set(row.key, normalize(row.value));
    return map;
  }, [rows]);

  const save = useCallback(
    async (entries: Record<string, string | number | boolean | null>) => {
      for (const [key, raw] of Object.entries(entries)) {
        const value = raw === null ? "null" : typeof raw === "string" ? raw : JSON.stringify(raw);
        const existing = rows.find((row) => row.key === key);
        if (existing) {
          if (normalize(existing.value) === normalize(value)) continue;
          await client.mutate({ mutation: EntityAppSettingUpdateDocument, variables: { id: existing.id, input: { value } } });
        } else {
          await client.mutate({ mutation: EntityAppSettingCreateDocument, variables: { input: { key, value, category } } });
        }
      }
      await refetch();
    },
    [category, client, refetch, rows],
  );

  return { values, rows, loading: loading && !data, error, save, refetch };
}

/** Strips JSON quoting so `"/data"` and `/data` compare equal, and treats `null` as empty. */
export function normalize(value: string): string {
  const trimmed = value.trim();
  if (trimmed === "null") return "";
  if (trimmed.length >= 2 && trimmed.startsWith('"') && trimmed.endsWith('"')) {
    try {
      return JSON.parse(trimmed) as string;
    } catch {
      return trimmed.slice(1, -1);
    }
  }
  return trimmed;
}

export const asBool = (value: string | undefined, fallback = false) => (value === undefined || value === "" ? fallback : value === "true");
export const asNumber = (value: string | undefined, fallback: number) => {
  const parsed = Number(value);
  return value === undefined || value === "" || Number.isNaN(parsed) ? fallback : parsed;
};

/** Reads a JSON array setting (or a comma list) as an array of strings. */
export const asList = (value: string | undefined): string[] => {
  if (!value) return [];
  const trimmed = value.trim();
  if (trimmed.startsWith("[")) {
    try {
      const parsed = JSON.parse(trimmed) as unknown;
      return Array.isArray(parsed) ? parsed.map(String) : [];
    } catch {
      return [];
    }
  }
  return trimmed.split(",").map((item) => item.trim()).filter(Boolean);
};
