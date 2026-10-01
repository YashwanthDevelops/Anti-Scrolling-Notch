export interface CapabilityEntry {
  id: string;
  surface: string;
  supportedVersion: string | null;
  transport: string;
  evidence: string;
  testReference: string;
  failureBehavior: string;
  fallback: string;
  adapterAvailable: boolean;
  enabled: boolean;
  disabledReason: string | null;
}

export interface CapabilityRegistry {
  schemaVersion: number;
  generation: string;
  runtimeSource: string;
  fixtureMode: boolean;
  installedCodexDesktopVersion: string | null;
  capabilities: CapabilityEntry[];
}

export function capabilityRequestGeneration(
  registry: CapabilityRegistry | null,
  capabilityId: string,
): string | null;
