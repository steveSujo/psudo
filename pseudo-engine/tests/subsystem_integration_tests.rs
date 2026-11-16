use pseudo_engine::engine::Engine;
use pseudo_engine::kv_cache::KVCachePrecision;
use pseudo_engine::sampler::{Sampler, SparseChannel};
use psudo::ownership::{BorrowChecker, CheckedEnvironment};
use psudo::parser::CheckedInterpret;
use psudo::pir_parser::parse_pir;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub fn build_test_gguf_and_tokenizer(dir: &Path) -> (PathBuf, PathBuf) {
    let tok_path = dir.join("tokenizer.json");
    let tok_json = r#"{
        "version": "1.0",
        "truncation": null,
        "padding": null,
        "added_tokens": [],
        "normalizer": null,
        "pre_tokenizer": null,
        "post_processor": null,
        "decoder": null,
        "model": {
            "type": "BPE",
            "vocab": {
                "<unk>": 0,
                "</s/>": 1,
                "f": 2,
                "sum_pair": 3,
                "(": 4,
                "a": 5,
                "b": 6,
                ")": 7,
                "[": 8,
                "v": 9,
                "ans": 10,
                "+": 11,
                "r": 12,
                "]": 13,
                "<talk>": 351,
                "</talk>": 352,
                "<pir>": 353,
                "</pir>": 354,
                "<fim>": 355,
                "<zettel>": 356,
                "<error>": 357,
                "<ctrl_390>": 390,
                "<ctrl_391>": 391,
                "Hello": 401,
                "world": 402,
                "borrow": 403,
                "!z": 404
            },
            "merges": []
        }
    }"#;
    fs::write(&tok_path, tok_json).unwrap();

    let gguf_path = dir.join("model.gguf");
    let mut gguf_bytes = Vec::new();
    // Magic "GGUF"
    gguf_bytes.extend_from_slice(b"GGUF");
    // Version 3
    gguf_bytes.extend_from_slice(&3u32.to_le_bytes());
    // Tensor count: 2 (token_embd.weight, output.weight)
    gguf_bytes.extend_from_slice(&2u64.to_le_bytes());
    // Metadata count: 4
    gguf_bytes.extend_from_slice(&4u64.to_le_bytes());

    let write_str = |v: &str, buf: &mut Vec<u8>| {
        buf.extend_from_slice(&(v.len() as u64).to_le_bytes());
        buf.extend_from_slice(v.as_bytes());
    };

    // 1. general.architecture = "pseudo" (type 8 = string)
    write_str("general.architecture", &mut gguf_bytes);
    gguf_bytes.extend_from_slice(&8u32.to_le_bytes());
    write_str("pseudo", &mut gguf_bytes);

    // 2. pseudo.embedding_length = 16 (type 4 = u32)
    write_str("pseudo.embedding_length", &mut gguf_bytes);
    gguf_bytes.extend_from_slice(&4u32.to_le_bytes());
    gguf_bytes.extend_from_slice(&16u32.to_le_bytes());

    // 3. pseudo.block_count = 1 (type 4 = u32)
    write_str("pseudo.block_count", &mut gguf_bytes);
    gguf_bytes.extend_from_slice(&4u32.to_le_bytes());
    gguf_bytes.extend_from_slice(&1u32.to_le_bytes());

    // 4. pseudo.vocab_size = 410 (type 4 = u32)
    write_str("pseudo.vocab_size", &mut gguf_bytes);
    gguf_bytes.extend_from_slice(&4u32.to_le_bytes());
    gguf_bytes.extend_from_slice(&410u32.to_le_bytes());

    // Tensors info
    // 1. token_embd.weight: shape [410, 16], type 0 (F32)
    write_str("token_embd.weight", &mut gguf_bytes);
    gguf_bytes.extend_from_slice(&2u32.to_le_bytes()); // 2 dims
    gguf_bytes.extend_from_slice(&16u64.to_le_bytes()); // dim 0: d_model = 16
    gguf_bytes.extend_from_slice(&410u64.to_le_bytes()); // dim 1: vocab_size = 410
    gguf_bytes.extend_from_slice(&0u32.to_le_bytes()); // F32
    gguf_bytes.extend_from_slice(&0u64.to_le_bytes()); // offset 0

    // 2. output.weight: shape [16, 410], type 0 (F32)
    write_str("output.weight", &mut gguf_bytes);
    gguf_bytes.extend_from_slice(&2u32.to_le_bytes()); // 2 dims
    gguf_bytes.extend_from_slice(&410u64.to_le_bytes()); // dim 0: vocab_size = 410
    gguf_bytes.extend_from_slice(&16u64.to_le_bytes()); // dim 1: d_model = 16
    gguf_bytes.extend_from_slice(&0u32.to_le_bytes()); // F32
    let emb_size = 410 * 16 * 4;
    gguf_bytes.extend_from_slice(&(emb_size as u64).to_le_bytes()); // offset

    // Align to 32 bytes
    while gguf_bytes.len() % 32 != 0 {
        gguf_bytes.push(0);
    }

    let _data_start = gguf_bytes.len();
    let total_data = vec![0u8; emb_size * 2];
    gguf_bytes.extend_from_slice(&total_data);

    fs::write(&gguf_path, gguf_bytes).unwrap();
    (gguf_path, tok_path)
}

#[test]
fn test_gguf_mmap_tq2_0_prefill_and_kv_tier_fork_isolation() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (gguf, tok) = build_test_gguf_and_tokenizer(tmp.path());

    let mut engine = Engine::new(&gguf, Some(&tok)).expect("Failed to load engine");
    let prompt_tokens = vec![2, 3, 4, 5, 6, 7, 8]; // f sum_pair (a b) [
    let logits = engine
        .prefill(&prompt_tokens, true)
        .expect("Prefill failed");
    assert!(!logits.is_empty());

    let tier1_len = engine.kv_cache.current_len;
    let forked = engine.kv_cache.fork_sequence(tier1_len);
    assert_eq!(forked.precision, KVCachePrecision::FP32);
    assert_eq!(forked.current_len, tier1_len);
}

#[test]
fn test_engine_pir_generation_parses_and_executes_in_psudo_vm() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (gguf, tok) = build_test_gguf_and_tokenizer(tmp.path());

    let mut engine = Engine::new(&gguf, Some(&tok)).expect("Failed to load engine");
    let constraint = Box::new(engine.build_pir_constraint(true, Some("f sum_pair (a b) [")));
    let mut sampler = Sampler::new(0.0, 0.90, 40, 1.1, constraint);

    let mut _generated = String::new();
    let _metrics = engine
        .generate("f sum_pair (a b) [", &mut sampler, 16, |tok| {
            _generated.push_str(tok);
            _generated.push(' ');
        })
        .expect("Generation failed");

    let synthetic_pir = "f sum_pair (a b) [\n  v ans (+ a b)\n  r ans\n]\nv out (sum_pair 12 30)\n";
    let stmts = parse_pir(synthetic_pir).expect("Failed to parse synthesized pIR");

    let borrow_checker = Rc::new(RefCell::new(BorrowChecker::new()));
    let mut interpreter = CheckedInterpret {
        environment: Rc::new(RefCell::new(CheckedEnvironment::new(borrow_checker))),
        check_borrows: true,
    };
    for stmt in stmts {
        interpreter.execute(&stmt);
    }
    let out_val = interpreter.environment.borrow().get("out");
    assert_eq!(out_val, Some(psudo::parser::DataTypes::Number(42.0)));
}

#[test]
fn test_pir_decompiler_and_prompt_router() {
    let (channel, prompt) =
        pseudo_engine::pir_decompiler::route_prompt("fun to add three numbers", false);
    assert_eq!(channel, SparseChannel::Pir);
    assert!(prompt.contains("<pir>"));

    let decompiled = pseudo_engine::pir_decompiler::decompile_pir_to_psudo(
        "f add_three (a b c) [\n  v sum (+ (+ a b) c)\n  r sum\n]",
    );
    assert!(decompiled.is_some());
    let psudo_code = decompiled.unwrap();
    assert!(psudo_code.contains("fn add_three(a, b, c)"));

    let mut lexer = psudo::lexer::Lexer::new(psudo_code);
    let tokens = lexer.scan_tokens().expect("Failed to lex decompiled psudo");
    let mut parser = psudo::parser::Parser::new(tokens);
    let parsed = parser.parse();
    assert!(
        parsed.is_ok(),
        "Failed to parse decompiled psudo: {:?}",
        parsed
    );
}

#[test]
fn test_trained_gguf_coherent_code_and_talk_generation() {
    let mut gguf_path = Path::new("pseudo-33m-tq2_0.gguf");
    let fallback_path = Path::new("pseudo-lang/models/pseudo-33m-tq2_0.gguf");
    if !gguf_path.exists() && fallback_path.exists() {
        gguf_path = fallback_path;
    }
    let tok_path = Path::new("pseudo-colab/configs/tokenizer.json");
    if gguf_path.exists() && tok_path.exists() {
        let mut engine =
            Engine::new(gguf_path, Some(tok_path)).expect("Failed to load trained model");
        let constraint = Box::new(engine.build_pir_constraint(true, Some("f add_three (a b c) [")));
        let mut sampler = Sampler::new(0.0, 0.90, 40, 1.1, constraint);
        let mut gen = String::new();
        let _ = engine.generate("f add_three (a b c) [", &mut sampler, 16, |t| {
            gen.push_str(t)
        });
        assert!(!gen.contains("<ctrl_"));
        assert!(!gen.contains("<reserved_"));

        // Talk channel check
        let talk_constraint = Box::new(engine.build_channel_constraint(
            SparseChannel::Clarify,
            Some("Prompt: how does borrow checking work?\n<talk>\n"),
        ));
        let mut talk_sampler = Sampler::new(0.0, 0.90, 40, 1.1, talk_constraint);
        let mut talk_gen = String::new();
        let metrics = engine
            .generate(
                "Prompt: how does borrow checking work?\n<talk>\n",
                &mut talk_sampler,
                32,
                |t| talk_gen.push_str(t),
            )
            .expect("Talk generation failed");
        assert!(metrics.generated_tokens <= 32);
        assert!(!talk_gen.contains("<ctrl_"));
        assert!(!talk_gen.contains("<reserved_"));
    }
}
