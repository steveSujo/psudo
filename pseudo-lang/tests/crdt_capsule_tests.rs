use psudo::crdt::SymbolMapCRDT;
use psudo::vfs::WorkspaceCapsule;
use psudo::zettel::ZettelStore;

#[test]
fn test_crdt_patch_and_materialization() {
    let mut crdt = SymbolMapCRDT::new(10);

    // Apply patch 1: fn calculate_tax
    let body1 = "fn calculate_tax(subtotal, rate)\n    return (subtotal * rate) / 100".to_string();
    let hash1 = SymbolMapCRDT::compute_hash(&body1);
    crdt.apply_patch("calculate_tax".to_string(), 1, hash1, vec![], body1, false);

    // Apply patch 2: fn calculate_rebate
    let body2 = "fn calculate_rebate(amount)\n    return amount / 20".to_string();
    let hash2 = SymbolMapCRDT::compute_hash(&body2);
    crdt.apply_patch(
        "calculate_rebate".to_string(),
        1,
        hash2,
        vec![],
        body2,
        false,
    );

    // Assert deterministic materialization contains both functions
    let source = crdt.materialize_active_source();
    assert!(source.contains("fn calculate_tax"));
    assert!(source.contains("fn calculate_rebate"));
}

#[test]
fn test_crdt_concurrent_agent_merge_zero_conflicts() {
    // Agent A creates a capsule and adds calculate_tax
    let mut cap_a = WorkspaceCapsule::new_empty("app-a");
    cap_a
        .upsert_stub(
            "src/main.psudo",
            "calculate_tax",
            &["subtotal".into(), "rate".into()],
            "compute tax",
        )
        .unwrap();

    // Agent B creates a capsule from same base and adds calculate_rebate
    let mut cap_b = WorkspaceCapsule::new_empty("app-b");
    cap_b
        .upsert_stub(
            "src/main.psudo",
            "calculate_rebate",
            &["amount".into()],
            "compute rebate",
        )
        .unwrap();

    // Merge Agent B's delta into Agent A's capsule
    cap_a.merge_delta(&cap_b).unwrap();

    // Materialized source in Agent A must contain BOTH functions without conflict!
    let src = cap_a.get_file("src/main.psudo").unwrap();
    assert!(src.contains("calculate_tax"), "Agent A stub must survive");
    assert!(
        src.contains("calculate_rebate"),
        "Agent B stub must be merged without conflict"
    );
}

#[test]
fn test_crdt_tombstone_pruning() {
    let mut crdt = SymbolMapCRDT::new(2);
    let body = "fn temp_helper()\n    return 1".to_string();
    let hash = SymbolMapCRDT::compute_hash(&body);
    crdt.apply_patch("temp_helper".into(), 1, hash, vec![], body, false);

    // Delete symbol (tombstone)
    crdt.apply_patch("temp_helper".into(), 2, hash, vec![], "".into(), true);
    assert!(crdt.symbols.get("temp_helper").unwrap().is_tombstone);
    assert!(!crdt.materialize_active_source().contains("temp_helper"));

    // Advance ticks past TTL and prune
    crdt.advance_tick();
    crdt.advance_tick();
    crdt.advance_tick();
    assert!(
        crdt.symbols.get("temp_helper").is_none(),
        "Tombstone must be pruned after TTL"
    );
}

#[test]
fn test_zettel_paging_and_projection() {
    let mut crdt = SymbolMapCRDT::new(10);
    let zettels = ZettelStore::new_with_core_defaults();

    let body = "fn compute_total(a, b)\n    return a + b".to_string();
    crdt.apply_patch("compute_total".into(), 1, 100, vec![], body, false);

    let projector = psudo::projector::ContextProjector::new(400);
    let slice = projector
        .build_slice(&crdt, &zettels, "compute_total", "")
        .unwrap();

    assert!(slice.contains("CORE INVARIANTS"));
    assert!(slice.contains("compute_total"));
}
