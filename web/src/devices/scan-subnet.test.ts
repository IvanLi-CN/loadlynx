import { describe, expect, test } from "vitest";
import {
  buildSubnetPlanFromSeedIp,
  currentLanSeedFromHostname,
} from "./scan-subnet.ts";

describe("LAN subnet derivation", () => {
  test("derives a /24 from the current private IPv4 hostname", () => {
    const plan = buildSubnetPlanFromSeedIp("192.168.31.42");
    expect(plan.cidr).toBe("192.168.31.0/24");
    expect(plan.hosts).toHaveLength(254);
  });

  test.each([
    "loadlynx.local",
    "localhost",
    "203.0.113.5",
    "",
  ])("does not accept %s as a current LAN seed", (hostname) => {
    expect(currentLanSeedFromHostname(hostname)).toBeNull();
  });
});
