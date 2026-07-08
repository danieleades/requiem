//! The individual repository health checks.

use requiem_core::Directory;

use super::{LinkIssue, PathIssue, SuspectIssue};

/// Find files that are not at their canonical locations.
pub(super) fn paths(directory: &Directory) -> Vec<PathIssue> {
    let digits = directory.config().digits();
    directory
        .check_path_drift()
        .into_iter()
        .map(|(hrid, current_path, expected_path)| PathIssue {
            hrid: hrid.display(digits).to_string(),
            current_path,
            expected_path,
        })
        .collect()
}

/// Find stale parent HRIDs, circular dependencies, and broken references.
pub(super) fn links(directory: &Directory) -> Vec<LinkIssue> {
    let digits = directory.config().digits();
    let mut issues = Vec::new();

    // Check for stale parent HRIDs
    let stale_hrids = directory.check_hrid_drift();
    for hrid in stale_hrids {
        issues.push(LinkIssue::StaleHrid {
            child: hrid.display(digits).to_string(),
        });
    }

    // Check for circular dependencies
    let cycles = directory.detect_cycles();
    for cycle in cycles {
        let cycle_path: Vec<String> = cycle
            .iter()
            .map(|hrid| hrid.display(digits).to_string())
            .collect();
        issues.push(LinkIssue::CircularDependency { cycle: cycle_path });
    }

    // Check for broken references (parent UUIDs that don't exist)
    for req in directory.requirements() {
        for (parent_uuid, _parent_info) in &req.parents {
            if directory.find_by_uuid(*parent_uuid).is_none() {
                issues.push(LinkIssue::BrokenReference {
                    child: req.hrid.display(digits).to_string(),
                    parent_uuid: parent_uuid.to_string(),
                });
            }
        }
    }

    issues
}

/// Find links whose stored parent fingerprint no longer matches.
pub(super) fn suspect(directory: &Directory) -> Vec<SuspectIssue> {
    let suspect_links = directory.suspect_links();
    let digits = directory.config().digits();
    let mut issues = Vec::new();

    for link in suspect_links {
        issues.push(SuspectIssue {
            child: link.child_hrid.display(digits).to_string(),
            parent: link.parent_hrid.display(digits).to_string(),
        });
    }

    issues
}
