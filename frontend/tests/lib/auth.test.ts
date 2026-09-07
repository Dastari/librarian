import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import { hasRole, isAdmin, normalizeRole } from "../../src/lib/auth";

const root = resolve(__dirname, "../..");

describe("auth role helpers", () => {
  it("normalizes supported roles case-insensitively", () => {
    expect(normalizeRole("ADMIN")).toBe("admin");
    expect(normalizeRole(" member ")).toBe("member");
    expect(normalizeRole("owner")).toBeNull();
    expect(normalizeRole(undefined)).toBeNull();
  });

  it("treats admins as members but not members as admins", () => {
    expect(isAdmin({ role: "Admin" })).toBe(true);
    expect(isAdmin({ role: "member" })).toBe(false);
    expect(hasRole({ role: "admin" }, "member")).toBe(true);
    expect(hasRole({ role: "member" }, "admin")).toBe(false);
    expect(hasRole(null, "member")).toBe(false);
  });

  it("guards Settings navigation and route access with the admin helper", () => {
    const navbar = readFileSync(resolve(root, "src/components/Navbar.tsx"), "utf8");
    const settingsRoute = readFileSync(resolve(root, "src/routes/settings.tsx"), "utf8");

    expect(navbar).toContain('item.to !== "/settings" || isAdmin(user)');
    expect(settingsRoute).toContain("if (!isAdmin(context.auth.user))");
    expect(settingsRoute).toContain("throw redirect({ to: '/libraries' })");
  });
});
