use open_bsl::{BackgroundJobService, HostEnv, HostError, JobId, JobWaitOutcome};
use std::{rc::Rc, time::Duration};

#[test]
fn background_service_signatures_are_reachable_through_the_facade() {
    let _: fn(&HostEnv) -> Option<Rc<dyn BackgroundJobService>> = HostEnv::background_jobs;
    let _: fn(&dyn BackgroundJobService) -> bool = |service| service.is_ready();
    type Wait = fn(
        &dyn BackgroundJobService,
        &[JobId],
        Option<Duration>,
    ) -> Result<JobWaitOutcome, HostError>;
    let _: Wait = |service, ids, timeout| service.wait_terminal(ids, timeout);
}
