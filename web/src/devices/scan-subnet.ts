export interface SubnetPlan {
  cidr: string; // e.g. "192.168.1.0/24"
  hosts: string[]; // ["192.168.1.1", ..., "192.168.1.254"]
}

/**
 * Derives a /24 subnet plan from a single seed IP address.
 * Use this to restrict scanning to a local network segment.
 *
 * @param seedIp A valid IPv4 string, e.g. "192.168.1.100"
 * @returns A SubnetPlan object containing the CIDR and list of 254 host IPs.
 * @throws Error if the seed IP is invalid or not a private/local range (optional strictness).
 */
export function buildSubnetPlanFromSeedIp(seedIp: string): SubnetPlan {
  const parts = seedIp.split(".").map(Number);

  if (parts.length !== 4 || parts.some(Number.isNaN)) {
    throw new Error(`Invalid IPv4 address: "${seedIp}"`);
  }

  // Validate range 0-255 derived from typical IPv4 rules
  if (parts.some((p) => p < 0 || p > 255)) {
    throw new Error(`Invalid IPv4 address segment in "${seedIp}"`);
  }

  // Basic check for private ranges (RFC 1918) + Link-Local (RFC 3927) could be added here,
  // but for a generic LAN scanner helper, we might want to be permissive
  // or at least allow any valid unicast IPv4.
  // The requirement asks for "Subnet /24" logic.
  // 192.168.x.x, 10.x.x.x, 172.16-31.x.x are private.
  // 169.254.x.x is link-local.

  // Let's stick to valid IPv4 format check for now to avoid over-blocking valid lab setups.

  const [a, b, c] = parts;
  const base = `${a}.${b}.${c}`;
  const cidr = `${base}.0/24`;
  const hosts: string[] = [];

  // Generate .1 through .254
  for (let i = 1; i <= 254; i++) {
    hosts.push(`${base}.${i}`);
  }

  return { cidr, hosts };
}
