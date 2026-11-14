//! Pure identity policy; tests run on any host.

use agon_osd::DisplayInfo;

/// Interface paths are opaque: normalize case, but never reconstruct a PnP
/// instance ID by splitting a symbolic link or matching a friendly name.
pub(crate) fn canonical_interface_path(path: &str) -> Option<String> {
    if !path.starts_with(r"\\?\") || path.len() <= 4 || path.contains('\0') {
        return None;
    }
    Some(path.to_ascii_lowercase())
}

/// Neither API documents an ordering relationship between physical handles and
/// monitor interfaces. Only a complete, unambiguous one-to-one result is usable.
pub(crate) fn unique_monitor_path(physical_count: u32, paths: &[String]) -> Option<String> {
    if physical_count != 1 || paths.len() != 1 {
        return None;
    }
    canonical_interface_path(&paths[0])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Provider {
    Dxva2,
    Intel,
    Amd,
    Nvidia,
}

pub(crate) struct Identity<'a> {
    pub(crate) provider: Provider,
    pub(crate) info: &'a DisplayInfo,
    pub(crate) edid: Option<&'a [u8; 128]>,
}

fn identity_key(info: &DisplayInfo) -> Option<(u32, u32, u32)> {
    let serial = info.serial_number();
    if serial == 0 || serial == u32::MAX {
        return None;
    }
    Some((info.vendor_id(), info.model_id(), serial))
}

fn same_identity(left: &Identity<'_>, right: &Identity<'_>) -> bool {
    match (identity_key(left.info), identity_key(right.info)) {
        (Some(left), Some(right)) => left == right,
        _ => left.edid.is_some() && left.edid == right.edid,
    }
}

pub(crate) fn merge_groups(
    identities: &[Identity<'_>],
    diagnostic: &mut dyn FnMut(String),
) -> Vec<usize> {
    let mut groups: Vec<_> = (0..identities.len()).collect();
    let mut visited = vec![false; identities.len()];
    for start in 0..identities.len() {
        if visited[start] {
            continue;
        }
        let mut component = vec![start];
        visited[start] = true;
        let mut cursor = 0;
        while cursor < component.len() {
            let current = component[cursor];
            for candidate in 0..identities.len() {
                if !visited[candidate]
                    && same_identity(&identities[current], &identities[candidate])
                {
                    visited[candidate] = true;
                    component.push(candidate);
                }
            }
            cursor += 1;
        }
        // A repeated provider means multiple physical outputs, even for a strong
        // serial key. Require pairwise agreement too: EDID fallback is not transitive.
        let unambiguous = component.iter().enumerate().all(|(position, &left)| {
            component[position + 1..].iter().all(|&right| {
                identities[left].provider != identities[right].provider
                    && same_identity(&identities[left], &identities[right])
            })
        });
        if unambiguous {
            for index in component {
                groups[index] = start;
            }
        } else {
            diagnostic(format!(
                "ambiguous display identity across {} outputs; keeping all channels independent",
                component.len()
            ));
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merge_groups(identities: &[Identity<'_>]) -> Vec<usize> {
        super::merge_groups(identities, &mut |_| {})
    }

    #[test]
    fn ambiguous_identity_emits_diagnostic_without_merging() {
        let info = info(0);
        let identities = [
            identity(Provider::Dxva2, &info, Some(&[7; 128])),
            identity(Provider::Intel, &info, Some(&[7; 128])),
            identity(Provider::Intel, &info, Some(&[7; 128])),
        ];
        let mut diagnostics = Vec::new();
        let groups = super::merge_groups(&identities, &mut |warning| diagnostics.push(warning));
        assert_eq!(groups, [0, 1, 2]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("ambiguous display identity"));
    }

    fn path(instance: &str) -> String {
        format!(r"\\?\DISPLAY#ACM1234#{instance}#{{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}}")
    }

    #[test]
    fn canonicalizes_case_without_changing_path_structure() {
        let original = path("4&ABC&0&UID1");
        assert_eq!(
            canonical_interface_path(&original),
            Some(original.to_ascii_lowercase())
        );
    }

    #[test]
    fn rejects_names_registry_paths_and_embedded_nuls() {
        for value in [
            "Generic PnP Monitor",
            r"DISPLAY\ACM1234\instance",
            r"\\?\",
            "\\\\?\\DISPLAY#bad\0suffix",
        ] {
            assert_eq!(canonical_interface_path(value), None);
        }
    }

    #[test]
    fn accepts_only_one_to_one_mapping() {
        let paths = vec![path("one")];
        assert_eq!(
            unique_monitor_path(1, &paths),
            canonical_interface_path(&paths[0])
        );
        assert_eq!(unique_monitor_path(0, &paths), None);
        assert_eq!(unique_monitor_path(2, &paths), None);
        assert_eq!(unique_monitor_path(1, &[]), None);
    }

    #[test]
    fn identical_models_and_reordered_interfaces_remain_ambiguous() {
        let mut paths = vec![path("one"), path("two")];
        assert_eq!(unique_monitor_path(1, &paths), None);
        paths.reverse();
        assert_eq!(unique_monitor_path(1, &paths), None);
        assert_eq!(unique_monitor_path(2, &paths), None);
        assert_eq!(
            unique_monitor_path(1, &[paths[0].clone(), paths[0].clone()]),
            None
        );
    }

    #[test]
    fn different_instances_have_different_stable_paths() {
        assert_ne!(
            canonical_interface_path(&path("one")),
            canonical_interface_path(&path("two"))
        );
    }

    fn info(serial: u32) -> DisplayInfo {
        DisplayInfo::new(
            "id".into(),
            "AGON".into(),
            0x1234,
            0x5678,
            serial,
            "location".into(),
        )
    }

    fn identity<'a>(
        provider: Provider,
        info: &'a DisplayInfo,
        edid: Option<&'a [u8; 128]>,
    ) -> Identity<'a> {
        Identity {
            provider,
            info,
            edid,
        }
    }

    #[test]
    fn unique_cross_provider_strong_identity_merges_even_without_edid() {
        let info = info(42);
        assert_eq!(
            merge_groups(&[
                identity(Provider::Dxva2, &info, None),
                identity(Provider::Intel, &info, Some(&[1; 128])),
                identity(Provider::Amd, &info, Some(&[2; 128])),
            ]),
            [0, 0, 0]
        );
    }

    #[test]
    fn unique_edid_candidate_can_merge_without_serial() {
        let info = info(0);
        assert_eq!(
            merge_groups(&[
                identity(Provider::Dxva2, &info, Some(&[7; 128])),
                identity(Provider::Intel, &info, Some(&[7; 128])),
            ]),
            [0, 0]
        );
    }

    #[test]
    fn identical_edid_two_screens_are_not_greedily_merged() {
        let info = info(0);
        // Both one-to-many directions and both-sides duplicates, including reordered input.
        for providers in [
            vec![Provider::Dxva2, Provider::Intel, Provider::Intel],
            vec![Provider::Intel, Provider::Intel, Provider::Dxva2],
            vec![Provider::Dxva2, Provider::Dxva2, Provider::Intel],
            vec![
                Provider::Dxva2,
                Provider::Intel,
                Provider::Dxva2,
                Provider::Intel,
            ],
        ] {
            let identities: Vec<_> = providers
                .into_iter()
                .map(|provider| identity(provider, &info, Some(&[7; 128])))
                .collect();
            assert_eq!(
                merge_groups(&identities),
                (0..identities.len()).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn same_provider_outputs_remain_separate_even_with_strong_serial() {
        let info = info(42);
        assert_eq!(
            merge_groups(&[
                identity(Provider::Intel, &info, None),
                identity(Provider::Intel, &info, None),
                identity(Provider::Dxva2, &info, None),
            ]),
            [0, 1, 2]
        );
    }

    #[test]
    fn missing_edid_and_serial_do_not_match() {
        let info = info(0);
        assert_eq!(
            merge_groups(&[
                identity(Provider::Dxva2, &info, None),
                identity(Provider::Intel, &info, Some(&[0; 128])),
                identity(Provider::Amd, &info, None),
            ]),
            [0, 1, 2]
        );
    }

    #[test]
    fn conflicting_serials_and_nontransitive_edid_are_not_merged() {
        let first = info(42);
        let missing = info(u32::MAX);
        let second = info(43);
        assert_eq!(
            merge_groups(&[
                identity(Provider::Dxva2, &first, Some(&[7; 128])),
                identity(Provider::Intel, &missing, Some(&[7; 128])),
                identity(Provider::Amd, &second, Some(&[7; 128])),
            ]),
            [0, 1, 2]
        );
        let different_vendor =
            DisplayInfo::new("id".into(), "AGON".into(), 9, 0x5678, 42, "location".into());
        assert!(!same_identity(
            &identity(Provider::Dxva2, &first, None),
            &identity(Provider::Intel, &different_vendor, None)
        ));
    }
}
