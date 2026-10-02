use std::panic::{AssertUnwindSafe, catch_unwind};
use validated_thing_arena_layout_probe::{
    RetainedEdge, RetainedNode,
    staged::{Site, Storage},
};

const NODE: RetainedNode = RetainedNode {
    kind: 1,
    first_edge: 0,
    edge_count: 0,
    first_byte: 0,
    byte_count: 0,
};
const EDGE: RetainedEdge = RetainedEdge {
    target: 0,
    original_index: 0,
};
const SITES: [Site; 4] = [Site::Nodes, Site::Edges, Site::Bytes, Site::Frames];

#[test]
fn a_transferred_exclusive_frame_can_only_be_extracted_after_completion() {
    let mut value = 0_u32;
    {
        let mut storage = Storage::new(10_000, 10_000, 10_000, 10_000);
        storage.begin_grow(Site::Frames, 1).unwrap();
        storage.copy_one();
        storage.push_frame(&mut value);
        storage.begin_grow(Site::Frames, 2).unwrap();
        storage.copy_one();
        // Never extract or dereference both representations: rejection must
        // happen before the non-Copy frame leaves the old allocation.
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                storage.pop_frame();
            }))
            .is_err()
        );
        assert_eq!(storage.len(Site::Frames), 1);
        storage.copy_one();
        assert_eq!(storage.transferring(), None);
        *storage.pop_frame() += 1;
        assert_eq!(storage.len(Site::Frames), 0);
    }
    assert_eq!(value, 1);
}

#[test]
fn every_build_mutator_is_rejected_throughout_each_transfer() {
    let input = [1_u32, 2];
    for site in SITES {
        // Both before the first copy and after the last copy but before the
        // old block is released are still in-flight transfer boundaries.
        for copied in 0..=1 {
            let mut storage = Storage::new(10_000, 10_000, 10_000, 10_000);
            for arena in SITES {
                storage.begin_grow(arena, 2).unwrap();
                storage.copy_one();
            }
            storage.push_node(NODE);
            storage.push_edge(EDGE);
            storage.push_byte(7);
            storage.push_frame(input.iter());
            storage.begin_grow(site, 4).unwrap();
            for _ in 0..copied {
                storage.copy_one();
            }
            for operation in 0..15 {
                let rejected = catch_unwind(AssertUnwindSafe(|| match operation {
                    0 => {
                        let _ = storage.pop_frame();
                    }
                    1 => {
                        storage.frame_mut().next();
                    }
                    2 => storage.push_frame(input.iter()),
                    3 => storage.push_node(NODE),
                    4 => storage.push_edge(EDGE),
                    5 => storage.push_byte(8),
                    6 => storage.set_node(0, NODE),
                    7 => storage.set_edge(0, EDGE),
                    8 => storage.set_byte(0, 8),
                    9..=12 => storage.truncate(SITES[operation - 9], 0),
                    13 => storage.release_site(Site::Frames),
                    14 => storage.release_site(site),
                    _ => unreachable!(),
                }))
                .is_err();
                assert!(
                    rejected,
                    "site={site:?}, copied={copied}, operation={operation}"
                );
                for arena in SITES {
                    assert_eq!(storage.len(arena), 1);
                }
            }
            while storage.transferring().is_some() {
                storage.copy_one();
            }
            assert_eq!(storage.node(0), NODE);
            assert_eq!(storage.edge(0), EDGE);
            assert_eq!(storage.byte(0), 7);
            assert_eq!(storage.pop_frame().next(), Some(&1));
            assert_eq!(storage.len(Site::Frames), 0);
            storage.clear();
            assert_eq!(storage.live_bytes(), 0);
            assert_eq!(storage.allocations(), storage.releases());
        }
    }
}

#[test]
fn clear_can_abandon_an_exclusive_frame_at_each_transfer_boundary() {
    for copied in 0..=1 {
        let mut value = 0_u32;
        {
            let mut storage = Storage::new(10_000, 10_000, 10_000, 10_000);
            storage.begin_grow(Site::Frames, 1).unwrap();
            storage.copy_one();
            storage.push_frame(&mut value);
            storage.begin_grow(Site::Frames, 2).unwrap();
            for _ in 0..copied {
                storage.copy_one();
            }
            storage.clear();
            storage.clear();
            assert_eq!(storage.live_bytes(), 0);
            assert_eq!(storage.allocations(), storage.releases());
        }
        value += 1;
        assert_eq!(value, 1);
    }
}
