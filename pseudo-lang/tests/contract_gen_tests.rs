//! Contract generation tests

use psudo::contract_gen::Contract;

const SYNC_STORE_FIXTURE: &str = r#"class VaultStore < BaseStore
  fn syncLogs(dirPath, maxEntries)
    if (!isDir(dirPath))
      return false
    let files = listDir(dirPath)
    let count = len(files)
    for let i = 0
      i < count
      i = i + 1
      if (i >= maxEntries)
        todo("implement pagination chunking when count exceeds maxEntries")
      let raw = readFile(dirPath + "/" + files[i])
      push(this.cachedLogs, str(raw))
    super.syncLogs(dirPath, maxEntries)
    return true

  @stub("Encrypts cachedLogs buffer using 2-bit LUT key stream before disk flush")
  fn flushEncrypted(outPath)
"#;

#[test]
fn test_contract_gen_sync_store() {
    let contracts = Contract::generate_contracts(SYNC_STORE_FIXTURE, "sync_store.pseudo").unwrap();

    assert_eq!(contracts.len(), 2);

    // Find syncLogs contract
    let sync_logs = contracts
        .iter()
        .find(|c| c.prov.symbol == "syncLogs")
        .unwrap();
    assert_eq!(sync_logs.prov.class_context, "VaultStore");
    assert_eq!(sync_logs.prov.base_class, "BaseStore");
    assert_eq!(sync_logs.signature, "fn syncLogs(dirPath, maxEntries)");
    assert_eq!(
        sync_logs.static_impl_summary.status,
        "PARTIAL_IMPL_WITH_TODO"
    );
    assert_eq!(
        sync_logs.static_impl_summary.stub_directive,
        "implement pagination chunking when count exceeds maxEntries"
    );
    assert!(sync_logs
        .static_impl_summary
        .intent
        .contains("Method on VaultStore"));
    assert!(sync_logs
        .static_impl_summary
        .guards
        .contains("not sandbox path dirPath is a directory"));
    assert!(sync_logs
        .static_impl_summary
        .side_effects
        .contains("Mutates this.cachedLogs"));
    assert!(
        sync_logs.static_impl_summary.sandbox_io.contains("isDir")
            && sync_logs.static_impl_summary.sandbox_io.contains("listDir")
            && sync_logs
                .static_impl_summary
                .sandbox_io
                .contains("readFile")
    );
    assert!(sync_logs
        .static_impl_summary
        .dependencies
        .contains("super.syncLogs"));
    assert!(sync_logs.static_impl_summary.control_flow == "Single-loop (depth 1)");

    // Find flushEncrypted contract
    let flush_encrypted = contracts
        .iter()
        .find(|c| c.prov.symbol == "flushEncrypted")
        .unwrap();
    assert_eq!(flush_encrypted.prov.class_context, "VaultStore");
    assert_eq!(flush_encrypted.prov.base_class, "BaseStore");
    assert_eq!(flush_encrypted.signature, "fn flushEncrypted(outPath)");
    assert_eq!(flush_encrypted.static_impl_summary.status, "BLACK_BOX_STUB");
    assert_eq!(
        flush_encrypted.static_impl_summary.stub_directive,
        "Encrypts cachedLogs buffer using 2-bit LUT key stream before disk flush"
    );
    assert!(flush_encrypted
        .static_impl_summary
        .intent
        .contains("Method on VaultStore"));
    assert_eq!(
        flush_encrypted.static_impl_summary.guards,
        "None (unconditional entry)."
    );
    assert_eq!(
        flush_encrypted.static_impl_summary.side_effects,
        "Pure (local state only)."
    );
    assert_eq!(
        flush_encrypted.static_impl_summary.sandbox_io,
        "Zero filesystem operations."
    );
    assert_eq!(
        flush_encrypted.static_impl_summary.dependencies,
        "Leaf function (zero internal calls)."
    );
    assert_eq!(
        flush_encrypted.static_impl_summary.control_flow,
        "Linear (0 loops)"
    );
}

#[test]
fn test_contract_deterministic_hashes() {
    // Same input should produce identical hashes
    let contracts1 = Contract::generate_contracts(SYNC_STORE_FIXTURE, "sync_store.pseudo").unwrap();
    let contracts2 = Contract::generate_contracts(SYNC_STORE_FIXTURE, "sync_store.pseudo").unwrap();

    assert_eq!(
        contracts1[0].prov.content_hash,
        contracts2[0].prov.content_hash
    );
    assert_eq!(
        contracts1[1].prov.content_hash,
        contracts2[1].prov.content_hash
    );
}

#[test]
fn test_contract_yaml_output() {
    let contracts = Contract::generate_contracts(SYNC_STORE_FIXTURE, "sync_store.pseudo").unwrap();
    let yaml = Contract::format_contracts_yaml(&contracts);

    // Verify it's valid YAML and contains expected fields
    assert!(yaml.contains("self_ref:"));
    assert!(yaml.contains("media_type: pseudo_ast_stub"));
    assert!(yaml.contains("prov:"));
    assert!(yaml.contains("signature:"));
    assert!(yaml.contains("static_impl_summary:"));
    assert!(yaml.contains("status:"));
    assert!(yaml.contains("stub_directive:"));
}

#[test]
fn test_contract_json_output() {
    let contracts = Contract::generate_contracts(SYNC_STORE_FIXTURE, "sync_store.pseudo").unwrap();
    let json = Contract::format_contracts_json(&contracts);

    // Verify it's valid JSON and contains expected fields
    assert!(json.contains("\"self_ref\""));
    assert!(json.contains("\"media_type\": \"pseudo_ast_stub\""));
    assert!(json.contains("\"prov\""));
    assert!(json.contains("\"signature\""));
    assert!(json.contains("\"static_impl_summary\""));
    assert!(json.contains("\"status\""));
    assert!(json.contains("\"stub_directive\""));
}

#[test]
fn test_contract_free_function() {
    let source = r#"
fn add(a, b)
    return a + b
"#;

    let contracts = Contract::generate_contracts(source, "math.pseudo").unwrap();
    assert_eq!(contracts.len(), 1);

    let add = &contracts[0];
    assert_eq!(add.prov.class_context, "nil");
    assert_eq!(add.prov.base_class, "nil");
    assert_eq!(add.signature, "fn add(a, b)");
    assert_eq!(add.static_impl_summary.status, "IMPLEMENTED");
    assert_eq!(add.static_impl_summary.stub_directive, "None");
}

#[test]
fn test_contract_with_stub_decorator() {
    let source = r#"
@stub("Calculate tax from subtotal and rate")
fn calculate_tax(subtotal, rate)
"#;

    let contracts = Contract::generate_contracts(source, "tax.pseudo").unwrap();
    assert_eq!(contracts.len(), 1);

    let calc_tax = &contracts[0];
    assert_eq!(calc_tax.static_impl_summary.status, "BLACK_BOX_STUB");
    assert_eq!(
        calc_tax.static_impl_summary.stub_directive,
        "Calculate tax from subtotal and rate"
    );
}

#[test]
fn test_contract_unparenthesized_guard_and_toy_builtins() {
    let source = r#"
fn is_server_error(status)
    if status >= 500
        return true
    return false

fn process_logs(path)
    let logs = load_json(path)
    let kv = persistent_kv("incidents")
    if is_server_error(503)
        set(kv, "last", "down")
    return len(logs)
"#;

    let contracts = Contract::generate_contracts(source, "logs.psudo").unwrap();
    assert_eq!(contracts.len(), 2);

    let guard_fn = &contracts[0];
    assert!(
        guard_fn.static_impl_summary.guards.contains("Returns true immediately"),
        "Expected unparenthesized guard to be detected, got: {}",
        guard_fn.static_impl_summary.guards
    );

    let proc_fn = &contracts[1];
    assert!(
        proc_fn.static_impl_summary.sandbox_io.contains("load_json")
            && proc_fn.static_impl_summary.sandbox_io.contains("persistent_kv")
            && proc_fn.static_impl_summary.sandbox_io.contains("set"),
        "Expected toy builtins in sandbox_io, got: {}",
        proc_fn.static_impl_summary.sandbox_io
    );
    assert!(
        proc_fn.static_impl_summary.side_effects.contains("persistent_kv"),
        "Expected persistent_kv mutation in side_effects, got: {}",
        proc_fn.static_impl_summary.side_effects
    );
    assert!(
        proc_fn.static_impl_summary.dependencies.contains("is_server_error"),
        "Expected helper call is_server_error in dependencies, got: {}",
        proc_fn.static_impl_summary.dependencies
    );
}
