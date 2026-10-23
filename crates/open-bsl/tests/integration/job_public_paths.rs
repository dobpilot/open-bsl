//! Контракт прежнего публичного модуля `open_bsl::jobs`.

#![cfg(not(target_arch = "wasm32"))]

use open_bsl::jobs::{
    BackgroundJobConfig, BackgroundStateFactory, HostProfileId, JobIdSource, JobRuntime,
    JobTimeSource, ShutdownReport,
};
use open_bsl::{BslDate, HostError, JobId, JobSnapshotDto, StateBuilder};
use std::sync::Arc;
use std::time::Duration;

struct Sources;

impl JobTimeSource for Sources {
    fn wall_now(&self) -> Option<BslDate> {
        None
    }
}

impl JobIdSource for Sources {
    fn next_id(&self) -> JobId {
        JobId([0; 16])
    }
}

impl BackgroundStateFactory for Sources {
    fn configure(&self, builder: StateBuilder) -> Result<StateBuilder, String> {
        Ok(builder)
    }
}

#[test]
fn job_types_and_host_traits_keep_their_public_paths() {
    fn shared<T: Send + Sync>() {}
    fn profile<T: Copy + Clone + std::fmt::Debug + Eq + std::hash::Hash>() {}
    shared::<Sources>();
    profile::<HostProfileId>();
    assert!(BackgroundJobConfig::default().validate().is_ok());
    assert_eq!(Sources.wall_now(), None);
    assert_eq!(Sources.next_id(), JobId([0; 16]));
    assert_eq!(
        ShutdownReport {
            detached_workers: 0
        }
        .detached_workers,
        0
    );
}

#[test]
fn job_runtime_methods_keep_their_public_signatures() {
    let _: fn(&JobRuntime, JobId) -> Option<Arc<JobSnapshotDto>> = JobRuntime::snapshot;
    let _: fn(&JobRuntime) -> Vec<Arc<JobSnapshotDto>> = JobRuntime::snapshots;
    let _: fn(&JobRuntime, JobId) -> Result<(), HostError> = JobRuntime::cancel;
    let _: fn(&JobRuntime, Duration) -> ShutdownReport = JobRuntime::shutdown;
    let _ = JobRuntime::submit_by_name;
    let _ = JobRuntime::wait_terminal;
    let _ = JobRuntime::take_messages;
}
