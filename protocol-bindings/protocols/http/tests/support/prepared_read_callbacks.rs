use super::*;

fn bound(h: &Harness) -> ConsumerPropertyRead {
    h.rt.0.bind_property_read("level", &mut work()).unwrap()
}

#[test]
fn repeated_bound_reads_need_no_lookup_qualification_or_fixed_resolution() {
    let h = harness(1, None);
    assert!(h.probe.qualifications.load(Ordering::Relaxed) > 0);
    let read = bound(&h);
    assert_eq!(h.probe.resolutions.load(Ordering::Relaxed), 1);
    h.probe.qualifications.store(0, Ordering::Relaxed);
    for _ in 0..3 {
        // One native construction credit, no mapping/name planning credit.
        let mut budget = work()
            .with_remaining(WorkClass::PlanningItems, 0)
            .with_remaining(WorkClass::BindingPolls, 1);
        let mut call = read
            .read(&h.rt.1, Deadline::NONE, now(), &mut budget)
            .unwrap();
        let result = settle(&mut call);
        assert_eq!(result.result().unwrap().data().unwrap().body, b"7");
        let original = h.probe.resolved_destination.load(Ordering::Relaxed);
        assert_eq!(result.destination().as_ptr() as usize, original);
        assert_eq!(h.probe.native_destination.load(Ordering::Relaxed), original);
        assert_eq!(
            h.probe.permission_destination.load(Ordering::Relaxed),
            original
        );
        drop((result, call));
    }
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 3);
    assert_eq!(h.probe.validations.load(Ordering::Relaxed), 3);
    assert_eq!(h.probe.resolutions.load(Ordering::Relaxed), 1);
    assert_eq!(h.probe.qualifications.load(Ordering::Relaxed), 0);
    drop(read);
    dispose(h);
}

#[test]
fn fixed_destination_keeps_its_original_bank_across_independent_call_capacity() {
    let h = harness(1, None);
    let read = bound(&h);
    let original = h.bank.clone();
    let before = original
        .account(ResourceKind::EngineLiveBytesGlobalMax)
        .used();
    let work_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let calls = ConsumerCalls::try_new(work_bank.clone(), 1).unwrap();
    let mut call = read
        .read(&calls, Deadline::NONE, now(), &mut work())
        .unwrap();
    assert_eq!(
        original
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used(),
        before
    );
    assert_eq!(
        original
            .account(ResourceKind::PendingHandlerCallsGlobalMax)
            .used(),
        1
    );
    let result = settle(&mut call);
    let pointer = h.probe.resolved_destination.load(Ordering::Relaxed);
    drop((read, h, call, calls));
    assert_eq!(result.destination().as_ptr() as usize, pointer);
    assert!(
        original
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used()
            > 0
    );
    assert!(work_bank.account(ResourceKind::CleanupBytesMax).used() > 0);
    drop(result);
    clean(&original);
    clean(&work_bank);
}

#[test]
fn binding_refuses_short_credit_funding_and_reentrant_removal_without_leaks() {
    let h = harness(1, None);
    let baseline = usage(&h.bank);
    assert!(
        h.rt.0
            .bind_property_read(
                "level",
                &mut work().with_remaining(WorkClass::BindingPolls, 0)
            )
            .is_err()
    );
    assert_eq!(h.probe.resolutions.load(Ordering::Relaxed), 0);
    assert_eq!(usage(&h.bank), baseline);
    let account = h.bank.account(ResourceKind::EngineLiveBytesGlobalMax);
    let pressure = h
        .bank
        .reserve(
            ResourceKind::EngineLiveBytesGlobalMax,
            account.limit() - account.used(),
        )
        .unwrap();
    assert!(h.rt.0.bind_property_read("level", &mut work()).is_err());
    assert_eq!(h.probe.resolutions.load(Ordering::Relaxed), 0);
    drop(pressure);
    assert_eq!(usage(&h.bank), baseline);
    // The handle header fits, but the separately retained destination owner
    // does not. Its failed reservation must refund the partial header debit.
    let account = h.bank.account(ResourceKind::EngineLiveBytesGlobalMax);
    let header = core::mem::size_of::<ConsumerPropertyRead>() as u64 + 128;
    let pressure = h
        .bank
        .reserve(
            ResourceKind::EngineLiveBytesGlobalMax,
            account.limit() - account.used() - header,
        )
        .unwrap();
    let pressured = usage(&h.bank);
    assert!(h.rt.0.bind_property_read("level", &mut work()).is_err());
    assert_eq!(usage(&h.bank), pressured);
    assert_eq!(h.probe.resolutions.load(Ordering::Relaxed), 0);
    drop(pressure);
    assert_eq!(usage(&h.bank), baseline);
    let association = h.rt.clone();
    hook(Edge::Resolve, move || association.0.begin_close());
    assert!(matches!(
        h.rt.0.bind_property_read("level", &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 0);
    dispose(h);
}

#[test]
fn original_revisions_and_bounds_fence_admission_progress_and_unread_handoff() {
    for dependency in ["data", "native", "access", "bounds"] {
        for edge in [Edge::Access, Edge::Issue, Edge::Poll, Edge::Validate] {
            let h = harness(1, None);
            let read = bound(&h);
            let probe = h.probe.clone();
            hook(edge, move || match dependency {
                "data" => {
                    probe.data_revision.fetch_add(1, Ordering::AcqRel);
                }
                "native" => {
                    probe.native_revision.fetch_add(1, Ordering::AcqRel);
                }
                "access" => {
                    probe.access_revision.fetch_add(1, Ordering::AcqRel);
                }
                "bounds" => probe.changed_bounds.store(true, Ordering::Release),
                _ => unreachable!(),
            });
            match read.read(&h.rt.1, Deadline::NONE, now(), &mut work()) {
                Err(_) => assert_eq!(h.probe.starts.load(Ordering::Relaxed), 0),
                Ok(mut call) => {
                    let result = settle(&mut call);
                    assert!(result.result().is_err(), "{dependency}");
                    drop((result, call));
                }
            }
            // A new handle cannot bless the old definition's changed baseline.
            assert!(h.rt.0.bind_property_read("level", &mut work()).is_err());
            drop(read);
            dispose(h);
        }
    }
}

#[test]
fn unread_bound_result_rechecks_original_qualification_after_policy_callback() {
    for dependency in ["data", "native", "access"] {
        let h = harness(1, None);
        let read = bound(&h);
        let mut call = read
            .read(&h.rt.1, Deadline::NONE, now(), &mut work())
            .unwrap();
        h.rt.1.step(&mut cx(), now(), &mut work());
        assert_eq!(h.probe.validations.load(Ordering::Relaxed), 1);
        let probe = h.probe.clone();
        hook(Edge::Access, move || {
            let revision = match dependency {
                "data" => &probe.data_revision,
                "native" => &probe.native_revision,
                "access" => &probe.access_revision,
                _ => unreachable!(),
            };
            revision.fetch_add(1, Ordering::AcqRel);
        });
        let result = settle(&mut call);
        assert!(result.result().is_err());
        assert!(matches!(
            result.native_outcome(),
            ConsumerNativeOutcome::Returned(Ok(_))
        ));
        drop((result, call, read));
        dispose(h);
    }
}

#[test]
fn configuration_replacement_cannot_retarget_an_old_bound_read() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerOwner::try_new(bank.clone(), 1, 1).unwrap();
    let instance = owner.register_thing("urn:bound", &mut work()).unwrap();
    let (original, original_probe) =
        association_harness(1, None, bank.clone(), instance.consumer());
    let (original, calls) = Rc::try_unwrap(original).ok().unwrap();
    let read = original.bind_property_read("level", &mut work()).unwrap();
    drop(
        owner
            .set_configuration("urn:bound", "default", original, &mut work())
            .unwrap_or_else(|_| panic!()),
    );
    let mut call = read
        .read(&calls, Deadline::NONE, now(), &mut work())
        .unwrap();
    let (replacement, replacement_probe) =
        association_harness(1, None, bank.clone(), instance.consumer());
    let (replacement, other_calls) = Rc::try_unwrap(replacement).ok().unwrap();
    let next = replacement
        .bind_property_read("level", &mut work())
        .unwrap();
    drop(
        owner
            .set_configuration("urn:bound", "default", replacement, &mut work())
            .unwrap_or_else(|_| panic!()),
    );
    assert!(matches!(
        read.read(&calls, Deadline::NONE, now(), &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    let result = settle(&mut call);
    assert!(result.result().is_err());
    assert_eq!(
        result.destination().as_ptr() as usize,
        original_probe.resolved_destination.load(Ordering::Relaxed)
    );
    assert_eq!(original_probe.polls.load(Ordering::Relaxed), 0);
    let mut next_call = next
        .read(&other_calls, Deadline::NONE, now(), &mut work())
        .unwrap();
    assert!(settle(&mut next_call).result().is_ok());
    assert_eq!(replacement_probe.polls.load(Ordering::Relaxed), 1);
    owner.remove_thing("urn:bound", &mut work()).unwrap();
    assert!(
        next.read(&other_calls, Deadline::NONE, now(), &mut work())
            .is_err()
    );
    drop((
        result,
        call,
        next_call,
        read,
        next,
        calls,
        other_calls,
        instance,
        owner,
    ));
    clean(&bank);
}

#[test]
fn bound_handle_drop_is_passive_and_close_retains_original_late_result() {
    let h = harness(1, None);
    let read = bound(&h);
    drop(read);
    let read = bound(&h);
    let mut call = read
        .read(&h.rt.1, Deadline::NONE, now(), &mut work())
        .unwrap();
    drop(read);
    let association = h.rt.clone();
    hook(Edge::Validate, move || association.0.begin_close());
    let result = settle(&mut call);
    assert!(result.result().is_err());
    assert_eq!(
        result.destination().as_ptr() as usize,
        h.probe.resolved_destination.load(Ordering::Relaxed)
    );
    assert!(
        matches!(result.native_outcome(), ConsumerNativeOutcome::Returned(Ok(output))
        if output.data().unwrap().body == b"7")
    );
    let bank = h.bank.clone();
    drop((h, call));
    assert!(bank.account(ResourceKind::CleanupBytesMax).used() > 0);
    drop(result);
    clean(&bank);
}
