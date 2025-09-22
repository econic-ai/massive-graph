use super::*;

struct IdMph;
impl MphIndexer<u64> for IdMph { fn eval(&self, key: &u64) -> usize { *key as usize } }

#[test]
fn reserved_and_by_index_basic() {
    let reserved_keys: Arc<[u64]> = Arc::from([0u64, 1u64]);
    let reserved_vals: Arc<[Arc<&'static str>]> = Arc::from([Arc::from("a"), Arc::from("b")]);
    let mph_vals: Arc<[Arc<&'static str>]> = Arc::from([Arc::from("x"), Arc::from("y"), Arc::from("z")]);
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let delta_stream = Arc::new(SegStream::<DeltaOp<u64, &'static str>>::new());
    let idx: OptimisedIndex<u64, &'static str> = OptimisedIndex::new(snap, delta_stream);

    assert_eq!(idx.get_by_index(1).as_deref(), Some(&"y"));
    assert_eq!(idx.get_reserved_slot(0).as_deref(), Some(&"a"));
}

#[test]
fn reserved_overlay_upsert_visible() {
    // Arrange
    let reserved_keys: Arc<[u64]> = Arc::from([10u64]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([Arc::new(1u64)]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let snap = Snapshot { version: 1, reserved_keys: Arc::clone(&reserved_keys), reserved_vals: Arc::clone(&reserved_vals), mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));
    // Act: placeholder path – will fail until delta overlay is implemented
    assert_eq!(idx.get_reserved_slot(0).as_deref(), Some(&1));
    // TODO: upsert to delta reserved_keys[0] then expect get_reserved_slot == new value
}

#[test]
fn get_by_index_out_of_range_none() {
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from([Arc::new(7u64)]);
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));
    assert!(idx.get_by_index(99).is_none());
}

#[test]
fn get_returns_base_without_delta() {
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from([Arc::new(42u64), Arc::new(100u64)]);
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));
    assert_eq!(idx.get(&1).as_deref(), Some(&100));
}

#[test]
fn snapshot_publish_visibility() {
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from([Arc::new(1u64)]);
    let snap = Snapshot { version: 1, reserved_keys: Arc::clone(&reserved_keys), reserved_vals: Arc::clone(&reserved_vals), mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));
    assert_eq!(idx.get_by_index(0).as_deref(), Some(&1));

    // Publish a new snapshot with different value
    let mph_vals2: Arc<[Arc<u64>]> = Arc::from([Arc::new(2u64)]);
    let snap2 = Snapshot { version: 2, reserved_keys, reserved_vals, mph_vals: mph_vals2, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    idx.publish_snapshot(snap2);
    assert_eq!(idx.get_by_index(0).as_deref(), Some(&2));
}

// TODO (future):
// - delta upsert/delete overlay on get(key)
// - consolidation cut semantics with cursor
// - concurrency smoke (readers + delta writer)

// ------------------------
// TDD: Expected-to-fail tests for unimplemented features
// ------------------------

#[test]
fn upsert_then_get_returns_value() {
    // Arrange minimal index
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));
    // Act: upsert then get
    idx.upsert(7, 123);
    // Assert: should be Some(123) after implementation
    assert_eq!(idx.get(&7).as_deref(), Some(&123));
}

#[test]
fn upsert_sets_contains_key_true() {
    let snap = Snapshot { version: 1, reserved_keys: Arc::from([]), reserved_vals: Arc::from([]), mph_vals: Arc::from([]), mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));
    idx.upsert(1, 1);
    assert!(idx.contains_key(&1));
}

#[test]
fn remove_clears_key() {
    let snap = Snapshot { version: 1, reserved_keys: Arc::from([]), reserved_vals: Arc::from([]), mph_vals: Arc::from([]), mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));
    idx.upsert(2, 10);
    idx.remove(&2);
    assert!(idx.get(&2).is_none());
    assert!(!idx.contains_key(&2));
}

#[test]
fn reserved_overlay_upsert_wins_over_base_reserved_val() {
    // reserved slot 0 has value 5; upsert key (reserved_keys[0]) with 9 should be visible via get_reserved_slot(0)
    let reserved_keys: Arc<[u64]> = Arc::from([100]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([Arc::new(5)]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let snap = Snapshot { version: 1, reserved_keys: Arc::clone(&reserved_keys), reserved_vals: Arc::clone(&reserved_vals), mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));
    idx.upsert(reserved_keys[0], 9);
    // Use overlay variant to validate delta wins over base reserved value
    assert_eq!(idx.get_reserved_slot_with_overlay(0).as_deref(), Some(&9));
}

#[test]
fn delta_overrides_base_get() {
    // base at idx 3 = 77; upsert key=3 with 88; get(3) should be 88
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from(
        (0..5u64).map(|i| Arc::new(i * 11)).collect::<Vec<_>>().into_boxed_slice()
    );
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));
    idx.upsert(3, 88);
    assert_eq!(idx.get(&3).as_deref(), Some(&88));
}

// ------------------------
// Stream-applied overlay behavior
// ------------------------

#[test]
fn stream_applier_upsert_then_get() {
    // index with empty base
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));

    let mut cur = idx.create_delta_cursor();
    idx.append_delta_upsert(11, 777);
    let applied = idx.apply_delta_once(&mut cur, 8);
    assert!(applied >= 1);
    assert_eq!(idx.get(&11).as_deref(), Some(&777));
}

#[test]
fn stream_applier_delete_tombstone() {
    // Base has a value for key 5; delta will tombstone it
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from((0..10u64).map(|i| Arc::new(i)).collect::<Vec<_>>().into_boxed_slice());
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));

    let mut cur = idx.create_delta_cursor();
    assert_eq!(idx.get(&5).as_deref(), Some(&5));
    idx.append_delta_delete(5);
    let _ = idx.apply_delta_once(&mut cur, 8);
    assert!(idx.get(&5).is_none());
}

#[test]
fn stream_applier_multiple_upserts_latest_wins() {
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: super::ArcIndexer(Arc::new(IdMph)) };
    let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegStream::new()));

    let mut cur = idx.create_delta_cursor();
    idx.append_delta_upsert(42, 1);
    idx.append_delta_upsert(42, 2);
    idx.append_delta_upsert(42, 3);
    let _ = idx.apply_delta_once(&mut cur, 16);
    assert_eq!(idx.get(&42).as_deref(), Some(&3));
}


