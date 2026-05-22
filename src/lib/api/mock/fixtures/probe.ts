import type { ProbeItem, SystemProbeReport } from "../../contracts";

const baseGenerated = "2026-05-22T10:00:00.000Z";

function item(
  p: Partial<ProbeItem> & Pick<ProbeItem, "id" | "group" | "status">,
): ProbeItem {
  return {
    nameKey: `probe.${p.id}.name`,
    messageKey: `probe.${p.id}.${p.status}`,
    value: null,
    fixAction: null,
    elapsedMs: 12,
    ...p,
  };
}

export function probeReportNewUser(): SystemProbeReport {
  const items: ProbeItem[] = [
    item({
      id: "os",
      group: "system",
      status: "green",
      value: {
        name: "Windows",
        version: "11",
        longVersion: "Windows 11 26100",
      },
    }),
    item({
      id: "arch",
      group: "system",
      status: "green",
      value: { arch: "x86_64", bits: 64 },
    }),
    item({
      id: "cpu",
      group: "system",
      status: "green",
      value: { physicalCores: 8, brand: "Intel" },
    }),
    item({
      id: "memoryTotal",
      group: "system",
      status: "green",
      value: { totalGb: 16 },
    }),
    item({
      id: "memoryAvailable",
      group: "system",
      status: "green",
      value: { availableGb: 9.2, percentFree: 57 },
    }),
    item({
      id: "disk",
      group: "system",
      status: "green",
      value: { availableGb: 120.4, mount: "C:\\" },
    }),
    item({
      id: "node",
      group: "runtime",
      status: "missing",
      value: null,
      fixAction: { kind: "installNode", targetLtsMajor: 20 },
    }),
    item({ id: "npm", group: "runtime", status: "missing", value: null }),
    item({
      id: "git",
      group: "runtime",
      status: "missing",
      value: null,
      fixAction: { kind: "installGit" },
    }),
    item({
      id: "path",
      group: "runtime",
      status: "yellow",
      value: { entries: [], missing: ["npm-global"] },
    }),
    item({
      id: "network",
      group: "network",
      status: "green",
      value: [
        { name: "npmjs", ok: true, latencyMs: 420 },
        { name: "npmmirror", ok: true, latencyMs: 180 },
      ],
    }),
    item({
      id: "envConflicts",
      group: "env",
      status: "green",
      value: { count: 0, conflicts: [] },
    }),
    item({
      id: "admin",
      group: "env",
      status: "green",
      value: { isAdmin: false },
    }),
    item({
      id: "psPolicy",
      group: "env",
      status: "green",
      value: { policy: "RemoteSigned" },
    }),
    item({ id: "systemProxy", group: "network", status: "green", value: {} }),
    item({
      id: "defender",
      group: "env",
      status: "unknown",
      value: { excluded: false },
      fixAction: {
        kind: "externalLink",
        url: "https://docs.microsoft.com/windows/security/threat-protection/microsoft-defender-antivirus/configure-exclusions-microsoft-defender-antivirus",
        labelKey: "probe.defender.docs",
      },
    }),
    item({
      id: "rosetta",
      group: "env",
      status: "unknown",
      value: { installed: false },
      fixAction: {
        kind: "externalLink",
        url: "https://support.apple.com/en-us/HT211861",
        labelKey: "probe.rosetta.docs",
      },
    }),
    item({
      id: "workdirWritable",
      group: "workdir",
      status: "green",
      value: { path: "C:\\Users\\you\\cc-launcher-projects", writable: true },
    }),
  ];
  return {
    overallStatus: "red",
    items,
    generatedAt: baseGenerated,
    probeVersion: 1,
  };
}

export function probeReportFullyConfigured(): SystemProbeReport {
  const items: ProbeItem[] = [
    item({
      id: "os",
      group: "system",
      status: "green",
      value: {
        name: "Windows",
        version: "11",
        longVersion: "Windows 11 26100",
      },
    }),
    item({
      id: "arch",
      group: "system",
      status: "green",
      value: { arch: "x86_64", bits: 64 },
    }),
    item({
      id: "cpu",
      group: "system",
      status: "green",
      value: { physicalCores: 8, brand: "Intel" },
    }),
    item({
      id: "memoryTotal",
      group: "system",
      status: "green",
      value: { totalGb: 16 },
    }),
    item({
      id: "memoryAvailable",
      group: "system",
      status: "green",
      value: { availableGb: 9.2, percentFree: 57 },
    }),
    item({
      id: "disk",
      group: "system",
      status: "green",
      value: { availableGb: 120.4, mount: "C:\\" },
    }),
    item({
      id: "node",
      group: "runtime",
      status: "green",
      value: {
        version: "v20.11.0",
        path: "C:\\Users\\you\\.cc-switch\\runtime\\node\\node.exe",
      },
    }),
    item({
      id: "npm",
      group: "runtime",
      status: "green",
      value: { version: "10.2.4" },
    }),
    item({
      id: "git",
      group: "runtime",
      status: "green",
      value: {
        version: "2.43.0",
        path: "C:\\Program Files\\Git\\bin\\git.exe",
      },
    }),
    item({
      id: "path",
      group: "runtime",
      status: "green",
      value: { entries: ["C:\\Users\\you\\.cc-switch\\runtime"], missing: [] },
    }),
    item({
      id: "network",
      group: "network",
      status: "green",
      value: [
        { name: "npmjs", ok: true, latencyMs: 380 },
        { name: "npmmirror", ok: true, latencyMs: 120 },
        { name: "tencent", ok: true, latencyMs: 220 },
        { name: "huawei", ok: true, latencyMs: 340 },
      ],
    }),
    item({
      id: "envConflicts",
      group: "env",
      status: "green",
      value: { count: 0, conflicts: [] },
    }),
    item({
      id: "admin",
      group: "env",
      status: "green",
      value: { isAdmin: false },
    }),
    item({
      id: "psPolicy",
      group: "env",
      status: "green",
      value: { policy: "RemoteSigned" },
    }),
    item({ id: "systemProxy", group: "network", status: "green", value: {} }),
    item({
      id: "defender",
      group: "env",
      status: "unknown",
      value: { excluded: true },
      fixAction: {
        kind: "externalLink",
        url: "https://docs.microsoft.com/windows/security/threat-protection/microsoft-defender-antivirus/configure-exclusions-microsoft-defender-antivirus",
        labelKey: "probe.defender.docs",
      },
    }),
    item({
      id: "rosetta",
      group: "env",
      status: "unknown",
      value: { installed: true },
      fixAction: {
        kind: "externalLink",
        url: "https://support.apple.com/en-us/HT211861",
        labelKey: "probe.rosetta.docs",
      },
    }),
    item({
      id: "workdirWritable",
      group: "workdir",
      status: "green",
      value: { path: "C:\\Users\\you\\cc-launcher-projects", writable: true },
    }),
  ];
  return {
    overallStatus: "green",
    items,
    generatedAt: baseGenerated,
    probeVersion: 1,
  };
}

export function probeReportNetworkFailure(): SystemProbeReport {
  const items = probeReportFullyConfigured().items.map((it) => {
    if (it.id === "network") {
      return {
        ...it,
        status: "red" as const,
        value: [
          { name: "npmjs", ok: false, latencyMs: 5000 },
          { name: "npmmirror", ok: false, latencyMs: 5000 },
          { name: "tencent", ok: false, latencyMs: 5000 },
          { name: "huawei", ok: false, latencyMs: 5000 },
        ],
      };
    }
    return it;
  });
  return {
    overallStatus: "red",
    items,
    generatedAt: baseGenerated,
    probeVersion: 1,
  };
}
