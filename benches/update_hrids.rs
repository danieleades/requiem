//! This bench test simulates updating the human-readable IDs (HRIDs) in parent
//! links in a large directory of requirements.

#![allow(missing_docs)]

use std::{num::NonZeroUsize, path::PathBuf};

use criterion::{criterion_group, criterion_main, Criterion};
use requiem::{domain::hrid::KindString, Directory, Hrid, Requirement};
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
        let sys_hrid = Hrid::new(sys_kind.clone(), id);
        let usr_hrid = Hrid::new(usr_kind.clone(), id);
        let requirement_hrid = {
            let requirement = directory.link_requirement(&sys_hrid, &usr_hrid).unwrap();
            requirement.hrid.clone()
        };
        directory.flush().unwrap();

        let mut on_disk = Requirement::load(
            directory.root(),
            requirement_hrid.clone(),
            directory.config(),
        )
        .unwrap();
        on_disk.parents_mut().next().unwrap().1.hrid = Hrid::try_from("WRONG-001").unwrap();
        on_disk.save(directory.root(), directory.config()).unwrap();
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
                let mut directory = Directory::new(tmp_dir.path().to_path_buf()).unwrap();
                directory.update_hrids();
                directory.flush().unwrap();
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, update_hrids);
criterion_main!(benches);
