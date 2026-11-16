use pseudo_engine::engine::Engine;
use pseudo_engine::kv_cache::{KVCache, KVCachePrecision};
use pseudo_engine::ops::{
    apply_rope, attention_causal_gqa, dequantize_turboquant_h_row, fwht, matmul_f32, matmul_tq2_0,
    matvec_f32, rmsnorm, rmsnorm_add_residual, softmax, swiglu,
};
use pseudo_engine::sampler::{
    LogitConstraint, PdaState, PirPdaConstraint, Sampler, SparseChannel, TokenClass,
};
use pseudo_engine::tensor::Tensor;

#[test]
fn test_tensor_creation() {
    let t = Tensor::zeros(vec![2, 4]);
    assert_eq!(t.numel(), 8);
    assert_eq!(t.shape, vec![2, 4]);
    assert_eq!(t.data.len(), 8);
}

#[test]
fn test_rmsnorm() {
    let mut out = vec![0.0; 4];
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let weight = vec![1.0, 1.0, 1.0, 1.0];
    rmsnorm(&mut out, &x, &weight, 1e-5);
    assert_eq!(out.len(), 4);
    let sum_sq: f32 = out.iter().map(|&v| v * v).sum();
    assert!((sum_sq / 4.0 - 1.0).abs() < 1e-3);
}

#[test]
fn test_rmsnorm_add_residual() {
    let mut out = vec![0.0; 4];
    let mut residual = vec![1.0, 1.0, 1.0, 1.0];
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let weight = vec![1.0, 1.0, 1.0, 1.0];
    rmsnorm_add_residual(&mut out, &mut residual, &x, &weight, 1e-5);
    // residual is now [1, 2, 3, 4]
    assert_eq!(residual, vec![1.0, 2.0, 3.0, 4.0]);
    let sum_sq: f32 = out.iter().map(|&v| v * v).sum();
    assert!((sum_sq / 4.0 - 1.0).abs() < 1e-3);
}

#[test]
fn test_swiglu() {
    let mut out = vec![0.0; 2];
    let gate = vec![0.0, 1.0];
    let up = vec![2.0, 3.0];
    swiglu(&mut out, &gate, &up);
    assert_eq!(out.len(), 2);
    assert!((out[0]).abs() < 1e-5);
    assert!(out[1] > 0.0);
}

#[test]
fn test_softmax() {
    let mut logits = vec![1.0, 2.0, 3.0];
    softmax(&mut logits);
    let sum: f32 = logits.iter().sum();
    assert!((sum - 1.0).abs() < 1e-5);
    assert!(logits[2] > logits[1]);
    assert!(logits[1] > logits[0]);
}

#[test]
fn test_apply_rope() {
    let mut q = vec![1.0, 0.0, 0.0, 1.0];
    let mut k = vec![1.0, 0.0, 0.0, 1.0];
    apply_rope(&mut q, &mut k, 1, 2, 1, 1, 10000.0);
    assert_eq!(q.len(), 4);
    assert_eq!(k.len(), 4);
}

#[test]
fn test_matmul_f32() {
    let mut out = vec![0.0; 4];
    let a = vec![1.0, 2.0, 3.0, 4.0];
    let b = vec![1.0, 0.0, 0.0, 1.0];
    matmul_f32(&mut out, &a, &b, 2, 2, 2);
    assert_eq!(out, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_matvec_f32() {
    let mut out = vec![0.0; 2];
    let x = vec![1.0, 2.0];
    let weights = vec![1.0, 0.0, 0.0, 1.0];
    matvec_f32(&mut out, &x, &weights, 2, 2);
    assert_eq!(out, vec![1.0, 2.0]);
}

#[test]
fn test_matmul_tq2_0() {
    let mut out = vec![0.0; 2];
    let x = vec![1.0, 2.0, 3.0, 4.0];
    // row 0: w = [+1, -1, 0, +1] -> 0b01_00_10_01 = 0x49
    // row 1: w = [-1, -1, +1, 0] -> 0b00_01_10_10 = 0x1A
    let weights = vec![0x49, 0x1A];
    let scales = vec![2.0, 0.5];
    matmul_tq2_0(&mut out, &x, &weights, &scales, 4, 2);
    // row 0: (1*1 - 2*1 + 0*3 + 1*4) * 2.0 = 3.0 * 2.0 = 6.0
    // row 1: (-1*1 - 1*2 + 1*3 + 0*4) * 0.5 = 0.0 * 0.5 = 0.0
    assert!((out[0] - 6.0).abs() < 1e-5);
    assert!((out[1] - 0.0).abs() < 1e-5);
}

#[test]
fn test_fwht() {
    let original = vec![1.0, 2.0, 3.0, 4.0];
    let mut transformed = original.clone();
    fwht(&mut transformed);
    // FWHT matrix for N=4 normalized by 1/2:
    // [ 1  1  1  1 ]
    // [ 1 -1  1 -1 ]
    // [ 1  1 -1 -1 ]
    // [ 1 -1 -1  1 ]
    // transformed[0] = (1+2+3+4)/2 = 5.0
    // transformed[1] = (1-2+3-4)/2 = -1.0
    // transformed[2] = (1+2-3-4)/2 = -2.0
    // transformed[3] = (1-2-3+4)/2 = 0.0
    assert!((transformed[0] - 5.0).abs() < 1e-5);
    assert!((transformed[1] - (-1.0)).abs() < 1e-5);
    assert!((transformed[2] - (-2.0)).abs() < 1e-5);
    assert!((transformed[3] - 0.0).abs() < 1e-5);

    // Invert by applying FWHT again
    fwht(&mut transformed);
    for (o, r) in original.iter().zip(transformed.iter()) {
        assert!((o - r).abs() < 1e-5, "Expected {}, got {}", o, r);
    }
}

#[test]
fn test_dequantize_turboquant_h() {
    let mut out = vec![0.0; 4];
    // weights: [+1, +1, +1, +1] -> 0b01_01_01_01 = 0x55
    let packed = vec![0x55];
    dequantize_turboquant_h_row(&mut out, &packed, 2.0);
    // out before fwht: [1, 1, 1, 1]
    // fwht([1, 1, 1, 1]): [4/2, 0, 0, 0] = [2, 0, 0, 0]
    // scaled by 2.0: [4, 0, 0, 0]
    assert!((out[0] - 4.0).abs() < 1e-5);
    assert!((out[1] - 0.0).abs() < 1e-5);
    assert!((out[2] - 0.0).abs() < 1e-5);
    assert!((out[3] - 0.0).abs() < 1e-5);
}

#[test]
fn test_attention_causal_gqa() {
    let head_dim = 4;
    let n_heads = 2;
    let n_kv_heads = 1; // 2 query heads per 1 KV head (GQA)
    let mut out = vec![0.0; n_heads * head_dim];
    let q = vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0];

    // Cache for 2 tokens:
    // t=0: k=[1, 0, 0, 0], v=[2, 2, 2, 2]
    // t=1: k=[0, 1, 0, 0], v=[5, 5, 5, 5]
    let k_cache = vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let v_cache = vec![2.0, 2.0, 2.0, 2.0, 5.0, 5.0, 5.0, 5.0];

    // At pos=0 (causal mask: cannot attend to t=1)
    attention_causal_gqa(
        &mut out, &q, &k_cache, &v_cache, 0, n_heads, n_kv_heads, head_dim,
    );
    // Both query heads must only attend to t=0, so output is exactly v[0]
    assert!((out[0] - 2.0).abs() < 1e-5);
    assert!((out[1] - 2.0).abs() < 1e-5);
    assert!((out[4] - 2.0).abs() < 1e-5);
    assert!((out[5] - 2.0).abs() < 1e-5);
}

#[test]
fn test_kv_cache_fork() {
    let mut cache = KVCache::new(2, 64, 2, 8);
    let k_data = vec![0.5f32; 16];
    let v_data = vec![0.8f32; 16];
    cache.append(0, 0, &k_data, &v_data);
    cache.append(0, 1, &k_data, &v_data);

    let forked = cache.fork_sequence(2);
    assert_eq!(forked.current_len, 2);
    assert_eq!(forked.k[0][..16], cache.k[0][..16]);
}

#[test]
fn test_kv_cache_int8() {
    let mut cache = KVCache::new_with_precision(2, 64, 2, 8, KVCachePrecision::INT8);
    let k_data = vec![
        1.0, -0.5, 0.25, -0.125, 0.8, -0.9, 0.0, 0.5, -1.0, 0.5, -0.25, 0.125, -0.8, 0.9, 0.0, -0.5,
    ];
    let v_data = vec![2.0f32; 16];
    cache.append(0, 0, &k_data, &v_data);

    // Verify quantized i8 values are populated
    assert_eq!(cache.k_int8[0].len(), 64 * 2 * 8);
    assert_eq!(cache.k_scales[0].len(), 64 * 2);
    assert!(cache.k_scales[0][0] > 0.0);

    // Verify dequantization error is minimal (< 1% of scale)
    for (orig, dequant) in k_data.iter().zip(cache.k[0][..16].iter()) {
        assert!(
            (orig - dequant).abs() < 0.02,
            "Orig: {}, Dequant: {}",
            orig,
            dequant
        );
    }

    // Verify forking preserves INT8 buffers
    let forked = cache.fork_sequence(1);
    assert_eq!(forked.precision, KVCachePrecision::INT8);
    assert_eq!(forked.k_int8[0][..16], cache.k_int8[0][..16]);
    assert_eq!(forked.k_scales[0][0], cache.k_scales[0][0]);
}

#[test]
fn test_sampler_greedy_and_temperature() {
    let vocab_fn = |id: u32| format!("tok_{}", id);
    let constraint = Box::new(PirPdaConstraint::new(true, 10, &vocab_fn));
    let mut sampler = Sampler::new(0.2, 0.95, 40, 1.1, constraint);
    let mut logits = vec![0.1, 0.5, 2.5, 0.2];
    let recent = vec![1];
    let token = sampler.sample(&mut logits, &recent, &vocab_fn);
    assert!(token < 4);
}

#[test]
fn test_pda_token_classification_and_control_blocking() {
    let vocab_fn = |id: u32| {
        if id == 390 {
            "<ctrl_390>".to_string()
        } else if id == 391 {
            "<ctrl_391>".to_string()
        } else if id == 269 {
            "f".to_string()
        } else {
            format!("tok_{}", id)
        }
    };
    let classes = PirPdaConstraint::classify_vocab_static(500, &vocab_fn);
    assert_eq!(classes[390].0 & TokenClass::CONTROL, TokenClass::CONTROL);
    assert_eq!(classes[391].0 & TokenClass::CONTROL, TokenClass::CONTROL);

    let mut constraint = PirPdaConstraint::new(true, 500, &vocab_fn);
    let mut logits = vec![0.0; 500];
    constraint.apply_mask(&mut logits, &vocab_fn);
    assert_eq!(logits[390], -f32::INFINITY);
    assert_eq!(logits[391], -f32::INFINITY);
}

#[test]
fn test_pda_prompt_prescan_and_parseable_pir() {
    let vocab_fn = |id: u32| match id {
        0 => "<pir>".to_string(),
        1 => "</pir>".to_string(),
        2 => "f".to_string(),
        3 => "calculate_total".to_string(),
        4 => "(".to_string(),
        5 => "a".to_string(),
        6 => "b".to_string(),
        7 => ")".to_string(),
        8 => "[".to_string(),
        9 => "v".to_string(),
        10 => "res".to_string(),
        11 => "(".to_string(),
        12 => "+".to_string(),
        13 => "a".to_string(),
        14 => "b".to_string(),
        15 => ")".to_string(),
        16 => "r".to_string(),
        17 => "res".to_string(),
        18 => "]".to_string(),
        _ => format!("tok_{}", id),
    };
    let mut constraint = PirPdaConstraint::from_prompt(
        SparseChannel::Pir,
        30,
        &vocab_fn,
        "f calculate_total (a b) [",
    );
    assert_eq!(constraint.state, PdaState::StatementStart);
    assert_eq!(constraint.block_depth, 1);

    let tokens = vec![
        (9, "v"),
        (10, "res"),
        (11, "("),
        (12, "+"),
        (13, "a"),
        (14, "b"),
        (15, ")"),
        (16, "r"),
        (17, "res"),
    ];
    for (id, lexeme) in tokens {
        let mut logits = vec![0.0; 30];
        constraint.apply_mask(&mut logits, &vocab_fn);
        constraint.accept_token(id, lexeme);
    }
    constraint.accept_token(18, "]");
    assert!(constraint.is_done());

    let synthetic_pir = "f calculate_total (a b) [\n  v res (+ a b)\n  r res\n]\n";
    let parsed = psudo::pir_parser::parse_pir(synthetic_pir);
    assert!(parsed.is_ok(), "Parsed successfully: {:?}", parsed);
}

#[test]
fn test_route_prompt_multichannel_aspects() {
    let (ch1, pr1) =
        pseudo_engine::pir_decompiler::route_prompt("/ask how does borrow check work?", false);
    assert_eq!(ch1, SparseChannel::Clarify);
    assert!(pr1.contains("<clarify>"));

    let (ch2, pr2) = pseudo_engine::pir_decompiler::route_prompt("<ctx_fn>fn foo(</ctx_fn>", false);
    assert_eq!(ch2, SparseChannel::Pir);
    assert!(pr2.contains("<ctx_fn>"));

    let (ch3, pr3) = pseudo_engine::pir_decompiler::route_prompt(
        "<error>[src/foo.psudo:1:1] E_ARITY</error>\nf foo (a) [\n  r 0\n]",
        false,
    );
    assert_eq!(ch3, SparseChannel::Pir);
    assert!(pr3.contains("<error>"));

    let (ch4, pr4) = pseudo_engine::pir_decompiler::route_prompt("/zettel rate_limiter", false);
    assert_eq!(ch4, SparseChannel::Zettel);
    assert!(pr4.contains("/zettel rate_limiter"));
}

#[test]
fn test_zettel_channel_isolation_and_ctx_routing() {
    use pseudo_engine::sampler::LogitConstraint;

    let vocab_fn = |id: u32| match id {
        352 => "</talk>".to_string(),
        353 => "<pir>".to_string(),
        354 => "</pir>".to_string(),
        355 => "<fim>".to_string(),
        356 => "</fim>".to_string(),
        361 => "<zettel>".to_string(),
        362 => "</zettel>".to_string(),
        _ => format!("tok_{}", id),
    };

    // 1. Channel isolation: Pir, Talk, Fim assign -inf to </zettel> (362) and <zettel> (361)
    for ch in [SparseChannel::Pir, SparseChannel::Clarify] {
        let mut constraint = PirPdaConstraint::from_prompt(ch, 500, &vocab_fn, "prompt");
        let mut logits = vec![1.0f32; 500];
        constraint.apply_mask(&mut logits, &vocab_fn);
        assert!(
            logits[361].is_infinite() && logits[361] < 0.0,
            "Channel {:?} must mask <zettel> (361) to -inf",
            ch
        );
        assert!(
            logits[362].is_infinite() && logits[362] < 0.0,
            "Channel {:?} must mask </zettel> (362) to -inf",
            ch
        );
    }

    // 2. SparseChannel::Zettel terminates at 25 tokens cap
    let mut z_constraint = PirPdaConstraint::from_prompt(
        SparseChannel::Zettel,
        500,
        &vocab_fn,
        "/zettel rate_limiter",
    );
    for _ in 0..24 {
        z_constraint.accept_token(100, "word");
    }
    assert_ne!(z_constraint.state, PdaState::Done);
    z_constraint.accept_token(101, "limit");
    assert_eq!(z_constraint.state, PdaState::Done);
    let mut z_logits = vec![1.0f32; 500];
    z_constraint.apply_mask(&mut z_logits, &vocab_fn);
    // When Done, only EOS tokens are allowed
    assert!(z_logits[100].is_infinite() && z_logits[100] < 0.0);

    // 3. Routing with <ctx>...</ctx>
    let ctx_prompt = "<ctx>+z id:sym_a kind:SYMBOL scope:sym_a links:[] fact:\"valid\"</ctx>\n/zettel evaluate sym_a";
    let (ch_z, pr_z) = pseudo_engine::pir_decompiler::route_prompt(ctx_prompt, false);
    assert_eq!(ch_z, SparseChannel::Zettel);
    assert!(pr_z.contains("<ctx>+z id:sym_a"));
    assert!(pr_z.contains("/zettel evaluate sym_a"));

    let talk_prompt = "how do zettels work?";
    let (ch_t, pr_t) = pseudo_engine::pir_decompiler::route_prompt(talk_prompt, false);
    assert_eq!(ch_t, SparseChannel::Clarify);
    assert!(pr_t.contains("<clarify>"));
}

#[test]
fn test_pda_error_frame_prescan_and_channel_caps() {
    let vocab_fn = |id: u32| format!("tok_{}", id);
    let prompt_err = "<error>[src/add_two.psudo:4:5] E_ARITY: expected 2 args</error>\nf add_two (a) [\n  r 0\n]";
    let constraint = PirPdaConstraint::from_prompt(SparseChannel::Pir, 10, &vocab_fn, prompt_err);
    assert_eq!(constraint.block_depth, 0);
    assert_eq!(constraint.state, PdaState::ExpectPirOpen);
}
#[test]
fn test_engine_synthetic_generation() {
    let model_path = "./pseudo-lang/models/pseudo-33m-tq2_0.gguf";
    let tok_path = "./pseudo-colab/configs/tokenizer.json";
    if std::path::Path::new(model_path).exists() && std::path::Path::new(tok_path).exists() {
        let mut engine = Engine::new(model_path, Some(std::path::Path::new(tok_path)))
            .expect("Failed to load engine");
        let constraint = Box::new(engine.build_pir_constraint(false, Some("fn test() {}")));
        let mut sampler = Sampler::new(0.2, 0.95, 40, 1.1, constraint);
        let metrics = engine
            .generate("fn test() {}", &mut sampler, 5, |_token| {})
            .expect("Generation failed");
        assert!(metrics.generated_tokens <= 5);
    }
}
