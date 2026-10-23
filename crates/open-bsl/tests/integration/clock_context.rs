use open_bsl::{
    CallContext, Capability, Clock, ContextKind, HostEnv, RtError, RuntimeShapes, format_value,
};

struct Ticking(i64);
impl Clock for Ticking {
    fn unix_millis(&mut self) -> i64 {
        let current = self.0;
        self.0 += 1;
        current
    }
}

#[test]
fn the_component_clock_is_the_injected_host_clock_and_is_absent_in_a_reduced_context() {
    let mut env = HostEnv::process().with_clock(Ticking(41));
    let clock = env.clock();
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let context = CallContext::minimal(&mut shapes, format_value);
    assert!(matches!(
        context.unix_millis(),
        Err(RtError::CapabilityMissing {
            capability: Capability::Clock,
            path: ContextKind::Reduced,
        })
    ));
    let context = context.with_clock(&clock);
    assert_eq!(env.unix_millis(), 41);
    assert_eq!(context.unix_millis().unwrap(), 42);
    assert_eq!(env.unix_millis(), 43);
}
