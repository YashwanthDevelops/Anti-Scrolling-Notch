//! Backend-owned capability policy for production Codex actions.
//!
//! Registry values are discovered by this process and serialized to the WebView.
//! They are never deserialized from frontend input. The internal hook wire
//! version is not a Codex CLI version and does not enable production monitoring.

use serde::Serialize;

use crate::codex_navigation::{self, InstalledCodexPackage};

pub const REGISTRY_SCHEMA_VERSION: u8 = 1;
pub const OPEN_CODEX_CAPABILITY: &str = "codex.openApp";
pub const PERMISSION_DECISION_CAPABILITY: &str = "codex.permissionDecision";
pub const VERIFIED_CODEX_DESKTOP_VERSION: &str = "26.928.2636.0";
const VERIFIED_CODEX_PACKAGE_FAMILY: &str = codex_navigation::CODEX_PACKAGE_FAMILY_NAME;

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityEntry {
    pub id: String,
    pub surface: String,
    pub supported_version: Option<String>,
    pub transport: String,
    pub evidence: String,
    pub test_reference: String,
    pub failure_behavior: String,
    pub fallback: String,
    pub adapter_available: bool,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRegistry {
    pub schema_version: u8,
    pub generation: String,
    pub runtime_source: String,
    pub fixture_mode: bool,
    pub installed_codex_desktop_version: Option<String>,
    pub capabilities: Vec<CapabilityEntry>,
}

impl CapabilityRegistry {
    /// Inspect only the registered Codex package identity/version. Any query
    /// failure or ambiguity leaves all version-gated actions disabled.
    pub fn discover() -> Self {
        Self::from_runtime(codex_navigation::discover_installed_package(), true)
    }

    fn from_runtime(
        installed: Option<InstalledCodexPackage>,
        activation_adapter_available: bool,
    ) -> Self {
        let package_matches = installed.as_ref().is_some_and(|package| {
            package.family_name == VERIFIED_CODEX_PACKAGE_FAMILY
                && package.version == VERIFIED_CODEX_DESKTOP_VERSION
        });
        let open_enabled = package_matches && activation_adapter_available;
        let installed_version = installed.as_ref().map(|package| package.version.clone());
        let installed_family = installed
            .as_ref()
            .map(|package| package.family_name.as_str())
            .unwrap_or("unavailable");
        let version_component = installed_version.as_deref().unwrap_or("unknown");
        let generation = format!(
            "cap-v{REGISTRY_SCHEMA_VERSION}|family={installed_family}|desktop={version_component}|activation-adapter={activation_adapter_available}"
        );

        let mut capabilities = vec![CapabilityEntry {
            id: OPEN_CODEX_CAPABILITY.into(),
            surface: "Windows packaged OpenAI Codex desktop app".into(),
            supported_version: Some(VERIFIED_CODEX_DESKTOP_VERSION.into()),
            transport: "IApplicationActivationManager / observed stable AUMID".into(),
            evidence: "COMPAT-07 verified generic app activation on Windows 11 build 26200".into(),
            test_reference: "windows/src-tauri/src/codex_navigation.rs::activates_the_installed_stable_codex_app".into(),
            failure_behavior: "Re-discover package identity and version before activation; return an error if stale, missing, unknown, or activation fails.".into(),
            fallback: "Open Codex manually from the Windows Start menu.".into(),
            adapter_available: activation_adapter_available,
            enabled: open_enabled,
            disabled_reason: (!open_enabled).then(|| {
                if !activation_adapter_available {
                    "The Windows activation adapter is unavailable.".into()
                } else {
                    "The installed Codex package is missing, ambiguous, or outside the verified package family/version.".into()
                }
            }),
        }];

        capabilities.extend([
            disabled(CapabilityDefinition {
                id: "codex.cliLifecycleMonitor",
                surface: "Codex CLI observing hooks",
                supported_version: Some("0.157.1"),
                transport: "Hook stdin projection → versioned user named pipe",
                evidence: "Four event names were captured at the CLI boundary; fixture/replay evidence is not a live production monitor binding.",
                test_reference: "tests/compat/codex-hooks/captured-projections.jsonl; windows/codex-hook-contract/src/lib.rs",
                failure_behavior: "Reject unknown event discriminators and malformed internal messages without a Codex decision.",
                fallback: "Use Codex directly; generic Open Codex activation is available only when its exact package is verified.",
                disabled_reason: "No production hook-install context or Stage 5/6 monitor pipeline exists; wire version 1 does not identify the running CLI version.",
            }),
            disabled(CapabilityDefinition {
                id: "codex.desktopSessionMonitor",
                surface: "Codex Desktop session monitoring",
                supported_version: None,
                transport: "No verified transport",
                evidence: "No Desktop session or hook adapter has been probed.",
                test_reference: "docs/codex-compatibility.md — Desktop row",
                failure_behavior: "Do not query or infer session state.",
                fallback: "Use Codex directly.",
                disabled_reason: "Desktop session API/version and adapter are unverified.",
            }),
            disabled(CapabilityDefinition {
                id: "codex.managedSession",
                surface: "Managed Codex sessions and streamed output",
                supported_version: None,
                transport: "No App Server adapter",
                evidence: "Managed stdio App Server has not been probed.",
                test_reference: "docs/codex-compatibility.md — managed App Server row",
                failure_behavior: "Do not start a managed turn or expose chat controls.",
                fallback: "Use Codex directly.",
                disabled_reason: "No verified App Server protocol version or adapter.",
            }),
            disabled(CapabilityDefinition {
                id: PERMISSION_DECISION_CAPABILITY,
                surface: "Codex permission decisions",
                supported_version: None,
                transport: "No verified decision adapter",
                evidence: "PermissionRequest Allow/Deny and managed approvals are unverified.",
                test_reference: "docs/codex-compatibility.md — permission decision row",
                failure_behavior: "Return no decision; leave resolution to Codex.",
                fallback: "Resolve the request in Codex.",
                disabled_reason: "No verified request schema, response, expiry or fallback adapter.",
            }),
            disabled(CapabilityDefinition {
                id: "codex.toastActivation",
                surface: "Native notification activation",
                supported_version: None,
                transport: "Unpackaged AppNotificationManager probe",
                evidence: "The API accepted Show, but no visible toast or activation callback was observed.",
                test_reference: "docs/work-packets/stage-3-packet-3-windows-platform-probes.md — native toast results",
                failure_behavior: "Do not claim delivery or process notification activation.",
                fallback: "Open Codex directly.",
                disabled_reason: "Toast delivery, identity and click activation are runtime-unverified.",
            }),
            disabled(CapabilityDefinition {
                id: "codex.mixedDpiOverlay",
                surface: "Mixed-DPI overlay repositioning",
                supported_version: None,
                transport: "Tauri monitor/window geometry",
                evidence: "Only single-display 125% geometry was observed.",
                test_reference: "docs/work-packets/stage-3-packet-3-windows-platform-probes.md — DPI results",
                failure_behavior: "Do not claim heterogeneous-monitor transition or hotplug support.",
                fallback: "Keep the current single-display placement.",
                disabled_reason: "A second display with a different effective DPI was unavailable.",
            }),
            disabled(CapabilityDefinition {
                id: "codex.attachmentDelivery",
                surface: "Codex attachment delivery",
                supported_version: None,
                transport: "No Codex attachment adapter",
                evidence: "Explorer-to-WebView2 transport was not observed; Codex attachment delivery is not implemented.",
                test_reference: "docs/codex-compatibility.md — file drop row",
                failure_behavior: "Do not send files to a Codex session.",
                fallback: "Attach the file in Codex directly.",
                disabled_reason: "Transport and Codex attachment workflow are unverified.",
            }),
            disabled(CapabilityDefinition {
                id: "codex.githubWorkflow",
                surface: "GitHub pull request and checks workflow",
                supported_version: None,
                transport: "No GitHub adapter",
                evidence: "Stage 7 owns Git/GitHub data, authentication and polling.",
                test_reference: "docs/codex-compatibility.md — GitHub workflow row",
                failure_behavior: "Do not display or act on stale/unqueried PR state.",
                fallback: "Inspect the repository or GitHub directly.",
                disabled_reason: "No GitHub API/auth adapter or response fixture exists.",
            }),
        ]);

        Self {
            schema_version: REGISTRY_SCHEMA_VERSION,
            generation,
            runtime_source: "windows-package-discovery".into(),
            fixture_mode: false,
            installed_codex_desktop_version: installed_version,
            capabilities,
        }
    }

    fn capability(&self, id: &str) -> Option<&CapabilityEntry> {
        self.capabilities
            .iter()
            .find(|capability| capability.id == id)
    }
}

struct CapabilityDefinition<'a> {
    id: &'a str,
    surface: &'a str,
    supported_version: Option<&'a str>,
    transport: &'a str,
    evidence: &'a str,
    test_reference: &'a str,
    failure_behavior: &'a str,
    fallback: &'a str,
    disabled_reason: &'a str,
}

fn disabled(definition: CapabilityDefinition<'_>) -> CapabilityEntry {
    CapabilityEntry {
        id: definition.id.into(),
        surface: definition.surface.into(),
        supported_version: definition.supported_version.map(str::to_owned),
        transport: definition.transport.into(),
        evidence: definition.evidence.into(),
        test_reference: definition.test_reference.into(),
        failure_behavior: definition.failure_behavior.into(),
        fallback: definition.fallback.into(),
        adapter_available: false,
        enabled: false,
        disabled_reason: Some(definition.disabled_reason.into()),
    }
}

/// Execute an action only when the current backend snapshot matches the
/// frontend's generation and the named capability has a verified adapter.
pub fn execute<T>(
    current: &CapabilityRegistry,
    expected_generation: &str,
    capability_id: &str,
    action: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    if expected_generation != current.generation {
        return Err("Codex capability snapshot is stale; refresh the app state and retry.".into());
    }
    if current.schema_version != REGISTRY_SCHEMA_VERSION
        || current.runtime_source != "windows-package-discovery"
        || current.fixture_mode
    {
        return Err("Codex capabilities are unavailable in this runtime mode.".into());
    }
    let capability = current
        .capability(capability_id)
        .ok_or_else(|| format!("Unsupported Codex capability: {capability_id}"))?;
    if !capability.enabled || !capability.adapter_available {
        return Err(capability
            .disabled_reason
            .clone()
            .unwrap_or_else(|| "Codex capability is disabled.".into()));
    }
    action()
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn verified_package() -> InstalledCodexPackage {
        InstalledCodexPackage {
            family_name: VERIFIED_CODEX_PACKAGE_FAMILY.into(),
            version: VERIFIED_CODEX_DESKTOP_VERSION.into(),
        }
    }

    #[test]
    fn exact_verified_desktop_identity_enables_only_generic_activation() {
        let registry = CapabilityRegistry::from_runtime(Some(verified_package()), true);
        let open = registry.capability(OPEN_CODEX_CAPABILITY).unwrap();

        assert!(open.enabled);
        assert!(open.adapter_available);
        assert_eq!(open.supported_version.as_deref(), Some("26.928.2636.0"));
        assert!(registry
            .capabilities
            .iter()
            .filter(|capability| capability.id != OPEN_CODEX_CAPABILITY)
            .all(|capability| !capability.enabled));
        assert!(!registry.fixture_mode);
    }

    #[test]
    fn missing_unknown_or_wrong_package_identity_disables_activation() {
        let unknown_version = InstalledCodexPackage {
            family_name: VERIFIED_CODEX_PACKAGE_FAMILY.into(),
            version: "26.999.0.0".into(),
        };
        let wrong_family = InstalledCodexPackage {
            family_name: "Other.Package_123".into(),
            version: VERIFIED_CODEX_DESKTOP_VERSION.into(),
        };

        for registry in [
            CapabilityRegistry::from_runtime(None, true),
            CapabilityRegistry::from_runtime(Some(unknown_version), true),
            CapabilityRegistry::from_runtime(Some(wrong_family), true),
            CapabilityRegistry::from_runtime(Some(verified_package()), false),
        ] {
            assert!(!registry.capability(OPEN_CODEX_CAPABILITY).unwrap().enabled);
        }
    }

    #[test]
    fn stale_or_absent_capabilities_reject_before_running_the_action() {
        let previous = CapabilityRegistry::from_runtime(Some(verified_package()), true);
        let current = CapabilityRegistry::from_runtime(
            Some(InstalledCodexPackage {
                family_name: VERIFIED_CODEX_PACKAGE_FAMILY.into(),
                version: "26.999.0.0".into(),
            }),
            true,
        );
        let called = Cell::new(false);
        assert!(execute(
            &current,
            &previous.generation,
            OPEN_CODEX_CAPABILITY,
            || {
                called.set(true);
                Ok(())
            }
        )
        .is_err());
        assert!(!called.get());

        let missing_adapter = CapabilityRegistry::from_runtime(Some(verified_package()), false);
        assert!(execute(
            &missing_adapter,
            &missing_adapter.generation,
            OPEN_CODEX_CAPABILITY,
            || {
                called.set(true);
                Ok(())
            }
        )
        .is_err());
        assert!(!called.get());

        assert!(
            execute(&current, &current.generation, "codex.unlisted", || {
                called.set(true);
                Ok(())
            })
            .is_err()
        );
        assert!(!called.get());
    }

    #[test]
    fn managed_chat_and_attachment_delivery_stay_disabled_until_adapters_exist() {
        let current = CapabilityRegistry::from_runtime(Some(verified_package()), true);
        let action_called = Cell::new(false);

        for capability_id in ["codex.managedSession", "codex.attachmentDelivery"] {
            assert!(execute(&current, &current.generation, capability_id, || {
                action_called.set(true);
                Ok(())
            })
            .is_err());
        }

        assert!(!action_called.get());
    }

    #[test]
    fn current_verified_snapshot_can_execute_only_the_named_action() {
        let current = CapabilityRegistry::from_runtime(Some(verified_package()), true);
        let process_id = execute(&current, &current.generation, OPEN_CODEX_CAPABILITY, || {
            Ok(321)
        })
        .unwrap();
        assert_eq!(process_id, 321);
    }

    #[test]
    fn fixture_like_identity_cannot_authorize_or_be_supplied_as_registry_data() {
        let fixture_identity = InstalledCodexPackage {
            family_name: "Demo.OpenAI.Codex_0000000000000".into(),
            version: VERIFIED_CODEX_DESKTOP_VERSION.into(),
        };
        let registry = CapabilityRegistry::from_runtime(Some(fixture_identity), true);

        assert_eq!(registry.runtime_source, "windows-package-discovery");
        assert!(!registry.fixture_mode);
        assert!(!registry.capability(OPEN_CODEX_CAPABILITY).unwrap().enabled);
        assert!(execute(
            &registry,
            &registry.generation,
            OPEN_CODEX_CAPABILITY,
            || Ok(())
        )
        .is_err());

        let mut fixture_mode = CapabilityRegistry::from_runtime(Some(verified_package()), true);
        fixture_mode.fixture_mode = true;
        assert!(execute(
            &fixture_mode,
            &fixture_mode.generation,
            OPEN_CODEX_CAPABILITY,
            || Ok(())
        )
        .is_err());

        let mut demo_registry = CapabilityRegistry::from_runtime(Some(verified_package()), true);
        demo_registry.runtime_source = "demo-fixture".into();
        assert!(execute(
            &demo_registry,
            &demo_registry.generation,
            OPEN_CODEX_CAPABILITY,
            || Ok(())
        )
        .is_err());
    }

    #[test]
    #[ignore = "requires installed OpenAI.Codex package; read-only runtime discovery"]
    fn discovers_the_installed_stable_codex_package_identity() {
        let registry = CapabilityRegistry::discover();
        assert_eq!(
            registry.installed_codex_desktop_version.as_deref(),
            Some(VERIFIED_CODEX_DESKTOP_VERSION)
        );
        assert!(registry
            .generation
            .contains(&format!("family={VERIFIED_CODEX_PACKAGE_FAMILY}|")));
        assert!(registry.capability(OPEN_CODEX_CAPABILITY).unwrap().enabled);
        assert!(registry
            .capabilities
            .iter()
            .filter(|capability| capability.id != OPEN_CODEX_CAPABILITY)
            .all(|capability| !capability.enabled));
    }
}
