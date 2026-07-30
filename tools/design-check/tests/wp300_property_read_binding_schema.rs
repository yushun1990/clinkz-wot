#![allow(dead_code)]

use std::boxed::Box;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingId(u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Generation(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlanId(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlanSetGeneration(Generation);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingCompatibility([u8; 16]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RegistrationIdentity {
    binding: BindingId,
    generation: Generation,
    configuration: [u8; 32],
    compatibility: BindingCompatibility,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingLifetimeFootprint {
    items: u16,
    bytes: u32,
}

impl BindingLifetimeFootprint {
    const fn fits_within(self, admitted: Self) -> bool {
        self.items <= admitted.items && self.bytes <= admitted.bytes
    }
}

#[derive(Debug, Eq, PartialEq)]
struct WorkBudget {
    binding_polls: u16,
}

impl WorkBudget {
    const fn new(binding_polls: u16) -> Self {
        Self { binding_polls }
    }

    fn try_consume_poll(&mut self) -> bool {
        if self.binding_polls == 0 {
            return false;
        }
        self.binding_polls -= 1;
        true
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreparationVisibility {
    Hidden,
    VisibleBoundedBuffer { items: u16, bytes: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PropertyReadCapabilities;

#[derive(Debug, Eq, PartialEq)]
struct CompilerRegistration {
    compatibility: BindingCompatibility,
}

#[derive(Debug, Eq, PartialEq)]
struct RegistrationInput<S> {
    identity: RegistrationIdentity,
    compiler: CompilerRegistration,
    server: S,
    capabilities: PropertyReadCapabilities,
    footprint: BindingLifetimeFootprint,
    admitted: BindingLifetimeFootprint,
    visibility: PreparationVisibility,
}

#[derive(Debug, Eq, PartialEq)]
struct RegistrationRejection<S> {
    input: RegistrationInput<S>,
    reason: &'static str,
}

impl<S> RegistrationRejection<S> {
    fn into_input(self) -> RegistrationInput<S> {
        self.input
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StaticRegistration<S> {
    identity: RegistrationIdentity,
    compiler: CompilerRegistration,
    server: S,
    visibility: PreparationVisibility,
}

impl<S> StaticRegistration<S> {
    fn try_new(input: RegistrationInput<S>) -> Result<Self, RegistrationRejection<S>> {
        if input.identity.compatibility != input.compiler.compatibility {
            return Err(RegistrationRejection {
                input,
                reason: "compiler/execution compatibility mismatch",
            });
        }
        if !input.footprint.fits_within(input.admitted) {
            return Err(RegistrationRejection {
                input,
                reason: "lifetime footprint exceeds admission",
            });
        }
        Ok(Self {
            identity: input.identity,
            compiler: input.compiler,
            server: input.server,
            visibility: input.visibility,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RouteKey {
    binding: BindingId,
    binding_generation: Generation,
    route_generation: Generation,
    plan_set_generation: PlanSetGeneration,
    plan: PlanId,
    route: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RouteStage {
    Vacant,
    Preparing,
    Prepared,
    AwaitingReadiness,
    Ready,
    Active,
    CommittedClosed,
    Serving,
    Draining,
    Closed,
    Residual,
}

#[derive(Debug, Eq, PartialEq)]
struct RouteSlot {
    key: RouteKey,
    stage: RouteStage,
    readiness_polls: u8,
    request_live: bool,
}

impl RouteSlot {
    const fn new(key: RouteKey) -> Self {
        Self {
            key,
            stage: RouteStage::Vacant,
            readiness_polls: 0,
            request_live: false,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct PrepareInput {
    route: RouteKey,
}

#[derive(Debug, Eq, PartialEq)]
struct InputRejection<T> {
    input: T,
    reason: &'static str,
}

impl<T> InputRejection<T> {
    fn into_input(self) -> T {
        self.input
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Progress {
    Pending,
    Ready,
}

trait StaticPropertyReadServer {
    fn start_prepare(
        &mut self,
        input: PrepareInput,
        route: &mut RouteSlot,
    ) -> Result<Progress, InputRejection<PrepareInput>>;

    fn start_readiness(&mut self, route: &mut RouteSlot) -> Progress;

    fn poll_readiness(&mut self, route: &mut RouteSlot, budget: &mut WorkBudget) -> Progress;

    fn activate(&mut self, route: &mut RouteSlot) -> Result<(), &'static str>;

    fn commit(&mut self, route: &mut RouteSlot) -> Result<(), &'static str>;

    fn abort(&mut self, route: &mut RouteSlot);

    fn shutdown(&mut self, route: &mut RouteSlot, residual: bool);
}

#[derive(Debug, Eq, PartialEq)]
struct MockStaticServer {
    external_readiness_polls: u8,
}

impl StaticPropertyReadServer for MockStaticServer {
    fn start_prepare(
        &mut self,
        input: PrepareInput,
        route: &mut RouteSlot,
    ) -> Result<Progress, InputRejection<PrepareInput>> {
        if route.stage != RouteStage::Vacant || route.key != input.route {
            return Err(InputRejection {
                input,
                reason: "route identity or stage mismatch",
            });
        }
        route.stage = RouteStage::Preparing;
        route.readiness_polls = self.external_readiness_polls;
        route.stage = RouteStage::Prepared;
        Ok(Progress::Ready)
    }

    fn start_readiness(&mut self, route: &mut RouteSlot) -> Progress {
        assert_eq!(route.stage, RouteStage::Prepared);
        route.stage = RouteStage::AwaitingReadiness;
        if route.readiness_polls == 0 {
            route.stage = RouteStage::Ready;
            Progress::Ready
        } else {
            Progress::Pending
        }
    }

    fn poll_readiness(&mut self, route: &mut RouteSlot, budget: &mut WorkBudget) -> Progress {
        assert_eq!(route.stage, RouteStage::AwaitingReadiness);
        if !budget.try_consume_poll() {
            return Progress::Pending;
        }
        route.readiness_polls -= 1;
        if route.readiness_polls == 0 {
            route.stage = RouteStage::Ready;
            Progress::Ready
        } else {
            Progress::Pending
        }
    }

    fn activate(&mut self, route: &mut RouteSlot) -> Result<(), &'static str> {
        if route.stage != RouteStage::Ready {
            return Err("route is not ready");
        }
        route.stage = RouteStage::Active;
        Ok(())
    }

    fn commit(&mut self, route: &mut RouteSlot) -> Result<(), &'static str> {
        if route.stage != RouteStage::Active {
            return Err("route is not active");
        }
        route.stage = RouteStage::CommittedClosed;
        Ok(())
    }

    fn abort(&mut self, route: &mut RouteSlot) {
        assert!(matches!(
            route.stage,
            RouteStage::Prepared | RouteStage::AwaitingReadiness | RouteStage::Ready
        ));
        route.stage = RouteStage::Closed;
    }

    fn shutdown(&mut self, route: &mut RouteSlot, residual: bool) {
        assert!(matches!(
            route.stage,
            RouteStage::Active
                | RouteStage::CommittedClosed
                | RouteStage::Serving
                | RouteStage::Draining
        ));
        route.stage = if residual {
            RouteStage::Residual
        } else {
            RouteStage::Closed
        };
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ServingAuthority {
    thing_generation: Generation,
    plan_set_generation: PlanSetGeneration,
}

impl ServingAuthority {
    fn claim_route<'a>(
        &'a self,
        lease: &'a mut RouteAcceptLease,
    ) -> Result<RouteAcceptClaim<'a>, &'static str> {
        if lease.claimed
            || lease.thing_generation != self.thing_generation
            || lease.route.plan_set_generation != self.plan_set_generation
        {
            return Err("authority mismatch");
        }
        lease.claimed = true;
        Ok(RouteAcceptClaim {
            authority: self,
            lease,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
struct RouteAcceptLease {
    thing_generation: Generation,
    route: RouteKey,
    claimed: bool,
}

struct RouteAcceptClaim<'a> {
    authority: &'a ServingAuthority,
    lease: &'a mut RouteAcceptLease,
}

impl<'a> RouteAcceptClaim<'a> {
    fn into_permit(self) -> RouteActivationPermit<'a> {
        RouteActivationPermit {
            authority: self.authority,
            route: &self.lease.route,
        }
    }
}

struct RouteActivationPermit<'a> {
    authority: &'a ServingAuthority,
    route: &'a RouteKey,
}

#[derive(Debug, Eq, PartialEq)]
struct RouteResponseOpportunity {
    route: RouteKey,
    correlation: u32,
}

#[derive(Debug, Eq, PartialEq)]
struct RouteInboundRequest {
    route: RouteKey,
    correlation: u32,
    property: Box<str>,
    response: RouteResponseOpportunity,
}

#[derive(Debug, Eq, PartialEq)]
struct RouteInboundResponse {
    opportunity: RouteResponseOpportunity,
    payload: Box<[u8]>,
}

fn accept_property_read(
    route: &mut RouteSlot,
    permit: RouteActivationPermit<'_>,
) -> Result<RouteInboundRequest, &'static str> {
    if route.stage != RouteStage::CommittedClosed
        || permit.route != &route.key
        || permit.authority.plan_set_generation != route.key.plan_set_generation
    {
        return Err("stale or mismatched permit");
    }
    route.stage = RouteStage::Serving;
    route.request_live = true;
    Ok(RouteInboundRequest {
        route: route.key,
        correlation: 7,
        property: "temperature".into(),
        response: RouteResponseOpportunity {
            route: route.key,
            correlation: 7,
        },
    })
}

fn start_response(
    route: &mut RouteSlot,
    response: RouteInboundResponse,
    has_capacity: bool,
) -> Result<(), InputRejection<RouteInboundResponse>> {
    if !has_capacity
        || !route.request_live
        || response.opportunity.route != route.key
        || response.opportunity.correlation != 7
    {
        return Err(InputRejection {
            input: response,
            reason: "response rejected before acceptance",
        });
    }
    route.request_live = false;
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
struct CleanupTransferRequest {
    route: RouteKey,
    owner: u16,
}

#[derive(Debug, Eq, PartialEq)]
struct CleanupTransferEnvelope<T> {
    request: CleanupTransferRequest,
    work: T,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CleanupRecord {
    owner: u16,
}

#[derive(Debug, Eq, PartialEq)]
enum CleanupTransferAcceptance<T> {
    Accepted(CleanupRecord),
    Rejected(CleanupTransferEnvelope<T>),
}

trait CleanupTransferTarget<T> {
    fn try_accept(&mut self, transfer: CleanupTransferEnvelope<T>) -> CleanupTransferAcceptance<T>;
}

struct RejectingCleanupTarget;

impl<T> CleanupTransferTarget<T> for RejectingCleanupTarget {
    fn try_accept(&mut self, transfer: CleanupTransferEnvelope<T>) -> CleanupTransferAcceptance<T> {
        CleanupTransferAcceptance::Rejected(transfer)
    }
}

trait HostPropertyReadServer: Send + Sync {
    fn visibility(&self) -> PreparationVisibility;
    fn readiness_polls(&self) -> u8;
}

struct HostRegistration {
    identity: RegistrationIdentity,
    compiler: CompilerRegistration,
    server: Box<dyn HostPropertyReadServer>,
}

impl HostRegistration {
    fn try_new(
        input: RegistrationInput<Box<dyn HostPropertyReadServer>>,
    ) -> Result<Self, RegistrationRejection<Box<dyn HostPropertyReadServer>>> {
        if input.identity.compatibility != input.compiler.compatibility {
            return Err(RegistrationRejection {
                input,
                reason: "host compiler/execution compatibility mismatch",
            });
        }
        if !input.footprint.fits_within(input.admitted) {
            return Err(RegistrationRejection {
                input,
                reason: "host lifetime footprint exceeds admission",
            });
        }
        Ok(Self {
            identity: input.identity,
            compiler: input.compiler,
            server: input.server,
        })
    }
}

struct MockHostServer {
    visibility: PreparationVisibility,
    readiness_polls: u8,
}

impl HostPropertyReadServer for MockHostServer {
    fn visibility(&self) -> PreparationVisibility {
        self.visibility
    }

    fn readiness_polls(&self) -> u8 {
        self.readiness_polls
    }
}

fn identity(compatibility: BindingCompatibility) -> RegistrationIdentity {
    RegistrationIdentity {
        binding: BindingId(3),
        generation: Generation(4),
        configuration: [5; 32],
        compatibility,
    }
}

fn route_key() -> RouteKey {
    RouteKey {
        binding: BindingId(3),
        binding_generation: Generation(4),
        route_generation: Generation(6),
        plan_set_generation: PlanSetGeneration(Generation(8)),
        plan: PlanId(9),
        route: 10,
    }
}

#[test]
fn complete_registration_rejects_mismatch_and_returns_the_author_input() {
    let registration = RegistrationInput {
        identity: identity(BindingCompatibility([1; 16])),
        compiler: CompilerRegistration {
            compatibility: BindingCompatibility([2; 16]),
        },
        server: MockStaticServer {
            external_readiness_polls: 0,
        },
        capabilities: PropertyReadCapabilities,
        footprint: BindingLifetimeFootprint {
            items: 1,
            bytes: 64,
        },
        admitted: BindingLifetimeFootprint {
            items: 1,
            bytes: 64,
        },
        visibility: PreparationVisibility::Hidden,
    };
    let rejection =
        StaticRegistration::try_new(registration).expect_err("mismatch must be rejected");
    let returned = rejection.into_input();
    assert_eq!(
        returned.compiler.compatibility,
        BindingCompatibility([2; 16])
    );
    assert_eq!(returned.server.external_readiness_polls, 0);

    let host_input = RegistrationInput {
        identity: identity(BindingCompatibility([1; 16])),
        compiler: CompilerRegistration {
            compatibility: BindingCompatibility([2; 16]),
        },
        server: Box::new(MockHostServer {
            visibility: PreparationVisibility::Hidden,
            readiness_polls: 1,
        }) as Box<dyn HostPropertyReadServer>,
        capabilities: PropertyReadCapabilities,
        footprint: BindingLifetimeFootprint {
            items: 1,
            bytes: 64,
        },
        admitted: BindingLifetimeFootprint {
            items: 1,
            bytes: 64,
        },
        visibility: PreparationVisibility::Hidden,
    };
    let host_rejection = match HostRegistration::try_new(host_input) {
        Err(rejection) => rejection,
        Ok(_) => panic!("host mismatch must be rejected"),
    };
    let returned_host = host_rejection.into_input();
    assert_eq!(returned_host.server.readiness_polls(), 1);
}

#[test]
fn static_immediate_property_read_reaches_response_and_explicit_cleanup() {
    let compatibility = BindingCompatibility([3; 16]);
    let mut registration = StaticRegistration::try_new(RegistrationInput {
        identity: identity(compatibility),
        compiler: CompilerRegistration { compatibility },
        server: MockStaticServer {
            external_readiness_polls: 0,
        },
        capabilities: PropertyReadCapabilities,
        footprint: BindingLifetimeFootprint {
            items: 1,
            bytes: 64,
        },
        admitted: BindingLifetimeFootprint {
            items: 2,
            bytes: 128,
        },
        visibility: PreparationVisibility::Hidden,
    })
    .expect("registration must be complete");
    let key = route_key();
    let mut route = RouteSlot::new(key);
    assert_eq!(
        registration
            .server
            .start_prepare(PrepareInput { route: key }, &mut route),
        Ok(Progress::Ready)
    );
    assert_eq!(
        registration.server.start_readiness(&mut route),
        Progress::Ready
    );
    registration.server.activate(&mut route).expect("activate");
    registration.server.commit(&mut route).expect("commit");
    assert_eq!(route.stage, RouteStage::CommittedClosed);

    let authority = ServingAuthority {
        thing_generation: Generation(11),
        plan_set_generation: key.plan_set_generation,
    };
    let mut lease = RouteAcceptLease {
        thing_generation: Generation(11),
        route: key,
        claimed: false,
    };
    let permit = authority
        .claim_route(&mut lease)
        .expect("unique route claim")
        .into_permit();
    let request = accept_property_read(&mut route, permit).expect("accept");
    let response = RouteInboundResponse {
        opportunity: request.response,
        payload: [42].into(),
    };
    start_response(&mut route, response, true).expect("response");
    route.stage = RouteStage::Draining;
    registration.server.shutdown(&mut route, false);
    assert_eq!(route.stage, RouteStage::Closed);
    assert!(!route.request_live);
}

#[test]
fn external_readiness_zero_budget_and_response_rejection_preserve_ownership() {
    let key = route_key();
    let mut server = MockStaticServer {
        external_readiness_polls: 1,
    };
    let mut route = RouteSlot::new(key);
    server
        .start_prepare(PrepareInput { route: key }, &mut route)
        .expect("prepare");
    assert_eq!(server.start_readiness(&mut route), Progress::Pending);
    let mut zero = WorkBudget::new(0);
    assert_eq!(
        server.poll_readiness(&mut route, &mut zero),
        Progress::Pending
    );
    assert_eq!(route.readiness_polls, 1);
    let mut one = WorkBudget::new(1);
    assert_eq!(server.poll_readiness(&mut route, &mut one), Progress::Ready);
    server.activate(&mut route).expect("activate");
    server.commit(&mut route).expect("commit");

    let authority = ServingAuthority {
        thing_generation: Generation(12),
        plan_set_generation: key.plan_set_generation,
    };
    let mut lease = RouteAcceptLease {
        thing_generation: Generation(12),
        route: key,
        claimed: false,
    };
    let request = accept_property_read(
        &mut route,
        authority
            .claim_route(&mut lease)
            .expect("claim")
            .into_permit(),
    )
    .expect("request");
    let response = RouteInboundResponse {
        opportunity: request.response,
        payload: [1, 2, 3].into(),
    };
    let response = start_response(&mut route, response, false)
        .expect_err("capacity rejection")
        .into_input();
    assert_eq!(&*response.payload, &[1, 2, 3]);
    assert!(route.request_live);
}

#[test]
fn rejected_cleanup_transfer_returns_the_complete_work_object() {
    let key = route_key();
    let transfer = CleanupTransferEnvelope {
        request: CleanupTransferRequest {
            route: key,
            owner: 13,
        },
        work: Box::new(RouteSlot {
            key,
            stage: RouteStage::Residual,
            readiness_polls: 0,
            request_live: false,
        }),
    };
    let mut target = RejectingCleanupTarget;
    let returned = match target.try_accept(transfer) {
        CleanupTransferAcceptance::Rejected(transfer) => transfer,
        CleanupTransferAcceptance::Accepted(_) => panic!("rejecting target accepted work"),
    };
    assert_eq!(returned.request.owner, 13);
    assert_eq!(returned.work.stage, RouteStage::Residual);
}

#[test]
fn host_erasure_covers_immediate_and_external_readiness_without_dispatch() {
    let compatibility = BindingCompatibility([9; 16]);
    let immediate = HostRegistration::try_new(RegistrationInput {
        identity: identity(compatibility),
        compiler: CompilerRegistration { compatibility },
        server: Box::new(MockHostServer {
            visibility: PreparationVisibility::Hidden,
            readiness_polls: 0,
        }),
        capabilities: PropertyReadCapabilities,
        footprint: BindingLifetimeFootprint {
            items: 1,
            bytes: 64,
        },
        admitted: BindingLifetimeFootprint {
            items: 1,
            bytes: 64,
        },
        visibility: PreparationVisibility::Hidden,
    })
    .unwrap_or_else(|_| panic!("immediate host registration"));
    let external = HostRegistration::try_new(RegistrationInput {
        identity: identity(compatibility),
        compiler: CompilerRegistration { compatibility },
        server: Box::new(MockHostServer {
            visibility: PreparationVisibility::VisibleBoundedBuffer {
                items: 1,
                bytes: 128,
            },
            readiness_polls: 2,
        }),
        capabilities: PropertyReadCapabilities,
        footprint: BindingLifetimeFootprint {
            items: 1,
            bytes: 128,
        },
        admitted: BindingLifetimeFootprint {
            items: 1,
            bytes: 128,
        },
        visibility: PreparationVisibility::VisibleBoundedBuffer {
            items: 1,
            bytes: 128,
        },
    })
    .unwrap_or_else(|_| panic!("external host registration"));
    assert_eq!(immediate.server.readiness_polls(), 0);
    assert_eq!(external.server.readiness_polls(), 2);
    assert!(matches!(
        external.server.visibility(),
        PreparationVisibility::VisibleBoundedBuffer {
            items: 1,
            bytes: 128
        }
    ));
}
