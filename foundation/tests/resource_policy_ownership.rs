use clinkz_wot_foundation::{GatewayDefaultV1, ResourceLimits, StaticResourceProfile, WorkBudget};

trait AmbiguousIfCopy<Marker> {
    fn marker() {}
}

impl<T: ?Sized> AmbiguousIfCopy<()> for T {}
impl<T: Copy> AmbiguousIfCopy<u8> for T {}

trait AmbiguousIfClone<Marker> {
    fn marker() {}
}

impl<T: ?Sized> AmbiguousIfClone<()> for T {}
impl<T: Clone> AmbiguousIfClone<u8> for T {}

fn assert_clone<T: Clone>() {}

#[test]
fn resource_policy_and_budget_ownership_are_exact() {
    assert_clone::<ResourceLimits>();

    let _ = <ResourceLimits as AmbiguousIfCopy<_>>::marker;
    let _ = <WorkBudget as AmbiguousIfCopy<_>>::marker;
    let _ = <WorkBudget as AmbiguousIfClone<_>>::marker;

    let _: &'static ResourceLimits = GatewayDefaultV1::LIMITS;
    let _: &'static ResourceLimits = GatewayDefaultV1::limits();
}
