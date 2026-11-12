import { describe, expect, it } from "vitest";

import { castOf, crewGroups, crewLine, type CreditRow } from "../credits";

const person = (id: string, name: string) => ({ id, name });
const credits: CreditRow[] = [
  { id: "1", kind: "cast", guest: false, characterName: "Neo", creditOrder: 1, person: person("p1", "Keanu Reeves") },
  { id: "2", kind: "cast", guest: false, characterName: "Morpheus", creditOrder: 0, person: person("p2", "Laurence Fishburne") },
  { id: "3", kind: "cast", guest: true, characterName: "Oracle", creditOrder: 9, person: person("p3", "Gloria Foster") },
  { id: "4", kind: "crew", guest: false, department: "Writing", job: "Writer", creditOrder: 2, person: person("p4", "Lilly Wachowski") },
  { id: "5", kind: "crew", guest: false, department: "Directing", job: "Director", creditOrder: 1, person: person("p4", "Lilly Wachowski") },
  { id: "6", kind: "crew", guest: false, department: "Production", job: "Executive Producer", creditOrder: 3, person: person("p5", "Barrie Osborne") },
  { id: "7", kind: "crew", guest: false, department: "Production", job: "Producer", creditOrder: 4, person: person("p5", "Barrie Osborne") },
  { id: "8", kind: "crew", guest: false, department: "Costume & Make-Up", job: "Costume Design", creditOrder: 5, person: person("p6", "Kym Barrett") },
  { id: "9", kind: "cast", guest: false, characterName: "Neo (young)", creditOrder: 12, person: person("p1", "Keanu Reeves") },
];

describe("credits", () => {
  it("lists cast in billing order, one entry per person", () => {
    const cast = castOf(credits);
    expect(cast.map((entry) => entry.person.name)).toEqual(["Laurence Fishburne", "Keanu Reeves", "Gloria Foster"]);
    expect(cast[1]!.characters).toEqual(["Neo", "Neo (young)"]);
    expect(castOf(credits, { guest: true }).map((entry) => entry.person.name)).toEqual(["Gloria Foster"]);
    expect(castOf(credits, { guest: false })).toHaveLength(2);
  });

  it("groups crew with directors first and joins a person's jobs", () => {
    const groups = crewGroups(credits);
    expect(groups.map((group) => group.label)).toEqual(["Directed by", "Written by", "Produced by", "Costume & Make-Up"]);
    expect(groups[2]!.entries).toHaveLength(1);
    expect(groups[2]!.entries[0]!.jobs).toEqual(["Executive Producer", "Producer"]);
  });

  it("writes a short crew line", () => {
    expect(crewLine(credits)).toBe("Directed by Lilly Wachowski · Written by Lilly Wachowski");
    expect(crewLine([])).toBe("");
  });
});
