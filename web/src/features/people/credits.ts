/**
 * Grouping a title's credits for its page, and a person's credits for theirs. The shapes are
 * the generated `Credit` fields the documents in `graphql/documents/credits.graphql` select.
 */

export interface CreditPerson {
  id: string;
  name: string;
  profileUrl?: string | null;
  knownForDepartment?: string | null;
}

export interface CreditRow {
  id: string;
  kind: string;
  guest: boolean;
  department?: string | null;
  job?: string | null;
  characterName?: string | null;
  creditOrder?: number | null;
  episodeCount?: number | null;
  person?: CreditPerson | null;
}

/** One person in a crew group, with every job they did there ("Writer, Executive Producer"). */
export interface CrewEntry<T extends CreditRow = CreditRow> {
  person: CreditPerson;
  jobs: string[];
  credits: T[];
}

export interface CrewGroup<T extends CreditRow = CreditRow> {
  label: string;
  entries: CrewEntry<T>[];
}

const order = (credit: CreditRow) => credit.creditOrder ?? Number.MAX_SAFE_INTEGER;

/** Cast in billing order, one entry per person (their characters joined). */
export function castOf<T extends CreditRow>(credits: readonly T[], { guest }: { guest?: boolean } = {}): Array<{ person: CreditPerson; characters: string[]; credits: T[]; episodeCount?: number | null }> {
  const byPerson = new Map<string, { person: CreditPerson; characters: string[]; credits: T[]; episodeCount?: number | null }>();
  for (const credit of [...credits].sort((a, b) => order(a) - order(b))) {
    if (credit.kind !== "cast" || !credit.person) continue;
    if (guest !== undefined && credit.guest !== guest) continue;
    const entry = byPerson.get(credit.person.id) ?? { person: credit.person, characters: [], credits: [], episodeCount: null };
    if (credit.characterName && !entry.characters.includes(credit.characterName)) entry.characters.push(credit.characterName);
    if (credit.job && credit.department !== "Acting" && !entry.characters.includes(credit.job)) entry.characters.push(credit.job);
    entry.episodeCount = Math.max(entry.episodeCount ?? 0, credit.episodeCount ?? 0) || null;
    entry.credits.push(credit);
    byPerson.set(credit.person.id, entry);
  }
  return [...byPerson.values()];
}

/** Which crew group a job belongs to, in the order a page lists them. */
const GROUPS: Array<{ label: string; test: (credit: CreditRow) => boolean }> = [
  { label: "Created by", test: (credit) => credit.job === "Creator" },
  { label: "Directed by", test: (credit) => credit.job === "Director" },
  { label: "Written by", test: (credit) => credit.department === "Writing" },
  { label: "Produced by", test: (credit) => /producer/i.test(credit.job ?? "") },
  { label: "Music", test: (credit) => /composer|music/i.test(credit.job ?? "") },
  { label: "Camera and editing", test: (credit) => credit.department === "Camera" || credit.department === "Editing" },
];

/** Crew in groups (creators, directors, writers, producers, music, then everyone else by department). */
export function crewGroups<T extends CreditRow>(credits: readonly T[]): CrewGroup<T>[] {
  const groups = new Map<string, Map<string, CrewEntry<T>>>();
  const labels: string[] = [];
  for (const credit of [...credits].sort((a, b) => order(a) - order(b))) {
    if (credit.kind !== "crew" || !credit.person) continue;
    const label = GROUPS.find((group) => group.test(credit))?.label ?? credit.department ?? "Crew";
    if (!groups.has(label)) {
      groups.set(label, new Map());
      labels.push(label);
    }
    const people = groups.get(label)!;
    const entry = people.get(credit.person.id) ?? { person: credit.person, jobs: [], credits: [] };
    if (credit.job && !entry.jobs.includes(credit.job)) entry.jobs.push(credit.job);
    entry.credits.push(credit);
    people.set(credit.person.id, entry);
  }
  const rank = (label: string) => {
    const index = GROUPS.findIndex((group) => group.label === label);
    return index === -1 ? GROUPS.length : index;
  };
  return labels
    .sort((a, b) => rank(a) - rank(b))
    .map((label) => ({ label, entries: [...groups.get(label)!.values()] }));
}

/** "Directed by A · Written by B, C": the few crew facts a summary line has room for. */
export function crewLine(credits: readonly CreditRow[], limit = 3): string {
  return crewGroups(credits)
    .filter((group) => ["Created by", "Directed by", "Written by"].includes(group.label))
    .map((group) => `${group.label} ${group.entries.slice(0, limit).map((entry) => entry.person.name).join(", ")}`)
    .join(" · ");
}
