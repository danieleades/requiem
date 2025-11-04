//! This bench test simulates updating the human-readable IDs (HRIDs) in parent
//! links in a large directory of requirements.

#![allow(missing_docs)]

use std::{num::NonZeroUsize, path::PathBuf};

use criterion::{criterion_group, criterion_main, Criterion};
use requiem::{domain::hrid::KindString, Directory, Hrid};
use tempfile::TempDir;

/// Generates a large number of interlinked documents
fn preseed_directory(path: PathBuf) {
    let mut directory = Directory::new(path).unwrap();
    let sys_kind = KindString::new("SYS".to_string()).unwrap();
    let usr_kind = KindString::new("USR".to_string()).unwrap();
    for i in 1..=99 {
        directory.add_requirement("USR", String::new()).unwrap();
        directory.add_requirement("SYS", String::new()).unwrap();
        let id = NonZeroUsize::new(i).unwrap();
        let mut requirement = directory
            .link_requirement(
                Hrid::new(sys_kind.clone(), id),
                Hrid::new(usr_kind.clone(), id),
            )
            .unwrap();
        requirement.parents_mut().next().unwrap().1.hrid = Hrid::try_from("WRONG-001").unwrap();
    }
}

use criterion::BatchSize;

fn update_hrids(c: &mut Criterion) {
    c.bench_function("update hrids", |b| {
        b.iter_batched(
            || {
                // Setup: create directory with broken HRIDs
                let tmp_dir = TempDir::new().unwrap();
                preseed_directory(tmp_dir.path().to_path_buf());
                tmp_dir
            },
            |tmp_dir| {
                Directory::new(tmp_dir.path().to_path_buf())
                    .unwrap()
                    .update_hrids()
                    .unwrap();
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, update_hrids);
criterion_main!(benches);
