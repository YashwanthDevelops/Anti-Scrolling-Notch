/**
 * Frontend presentation guard for backend capability snapshots.
 * Rust repeats every check before executing a control; this only prevents an
 * unavailable capability from being presented as an actionable UI affordance.
 */
export function capabilityRequestGeneration(registry, capabilityId) {
  if (
    !registry ||
    registry.schemaVersion !== 1 ||
    registry.runtimeSource !== "windows-package-discovery" ||
    registry.fixtureMode !== false ||
    typeof registry.generation !== "string" ||
    registry.generation.length === 0 ||
    !Array.isArray(registry.capabilities)
  ) {
    return null;
  }

  const capability = registry.capabilities.find((entry) => entry.id === capabilityId);
  if (!capability || capability.enabled !== true || capability.adapterAvailable !== true) {
    return null;
  }
  return registry.generation;
}
