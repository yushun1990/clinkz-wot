#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArtifactRole {
    ConsumerCall,
    ProducerRoute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RouteReservationIdentity {
    collision_domain: [u8; 16],
    endpoint: [u8; 32],
}

#[derive(Debug, Eq, PartialEq)]
struct BindingArtifact<A> {
    role_reservation: Option<RouteReservationIdentity>,
    payload: A,
}

impl<A> BindingArtifact<A> {
    fn new(payload: A) -> Self {
        Self {
            role_reservation: None,
            payload,
        }
    }

    fn producer_route(reservation: RouteReservationIdentity, payload: A) -> Self {
        Self {
            role_reservation: Some(reservation),
            payload,
        }
    }

    fn route_reservation(&self) -> Option<RouteReservationIdentity> {
        self.role_reservation
    }

    fn into_route_parts(self) -> (Option<RouteReservationIdentity>, A) {
        (self.role_reservation, self.payload)
    }

    fn map_payload<B>(self, map: impl FnOnce(A) -> B) -> BindingArtifact<B> {
        BindingArtifact {
            role_reservation: self.role_reservation,
            payload: map(self.payload),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArtifactRejectionReason {
    MissingRouteReservation,
    UnexpectedRouteReservation,
}

#[derive(Debug, Eq, PartialEq)]
struct BindingArtifactEnvelope<A> {
    role: ArtifactRole,
    artifact: BindingArtifact<A>,
}

impl<A> BindingArtifactEnvelope<A> {
    fn try_new(
        role: ArtifactRole,
        artifact: BindingArtifact<A>,
    ) -> Result<Self, (ArtifactRejectionReason, BindingArtifact<A>)> {
        match (role, artifact.route_reservation()) {
            (ArtifactRole::ProducerRoute, None) => {
                return Err((ArtifactRejectionReason::MissingRouteReservation, artifact));
            }
            (ArtifactRole::ConsumerCall, Some(_)) => {
                return Err((
                    ArtifactRejectionReason::UnexpectedRouteReservation,
                    artifact,
                ));
            }
            _ => {}
        }
        Ok(Self { role, artifact })
    }

    fn route_reservation(&self) -> Option<RouteReservationIdentity> {
        self.artifact.route_reservation()
    }
}

fn reservation() -> RouteReservationIdentity {
    RouteReservationIdentity {
        collision_domain: [21; 16],
        endpoint: [22; 32],
    }
}

#[test]
fn producer_route_requires_and_exposes_compiler_metadata() {
    let envelope = BindingArtifactEnvelope::try_new(
        ArtifactRole::ProducerRoute,
        BindingArtifact::producer_route(reservation(), "mock://tank/level"),
    )
    .expect("complete Producer-route artifact");
    assert_eq!(envelope.role, ArtifactRole::ProducerRoute);
    assert_eq!(envelope.route_reservation(), Some(reservation()));
}

#[test]
fn missing_producer_route_reservation_fails_without_losing_artifact() {
    let (reason, artifact) = BindingArtifactEnvelope::try_new(
        ArtifactRole::ProducerRoute,
        BindingArtifact::new("mock://tank/level"),
    )
    .unwrap_err();
    assert_eq!(reason, ArtifactRejectionReason::MissingRouteReservation);
    assert_eq!(artifact.payload, "mock://tank/level");
}

#[test]
fn consumer_artifact_rejects_route_reservation_metadata() {
    let (reason, artifact) = BindingArtifactEnvelope::try_new(
        ArtifactRole::ConsumerCall,
        BindingArtifact::producer_route(reservation(), "mock://tank/level"),
    )
    .unwrap_err();
    assert_eq!(reason, ArtifactRejectionReason::UnexpectedRouteReservation);
    assert_eq!(artifact.route_reservation(), Some(reservation()));
}

#[test]
fn host_payload_erasure_preserves_route_reservation() {
    let artifact = BindingArtifact::producer_route(reservation(), 17_u8);
    let erased = artifact.map_payload(|payload| Box::new(payload) as Box<dyn Send + Sync>);
    assert_eq!(erased.route_reservation(), Some(reservation()));
}

#[test]
fn complete_consuming_surface_preserves_route_reservation() {
    let artifact = BindingArtifact::producer_route(reservation(), 17_u8);
    assert_eq!(artifact.into_route_parts(), (Some(reservation()), 17_u8));
}

#[test]
fn ordinary_artifact_constructor_cannot_smuggle_route_metadata() {
    let artifact = BindingArtifact::new(17_u8);
    assert_eq!(artifact.route_reservation(), None);
}
