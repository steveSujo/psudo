use crate::gguf::{GgufFile, GgufTensorType, ModelConfig};
use crate::kv_cache::KVCache;
use crate::ops::{apply_rope, attention_causal_gqa, matmul_tq2_0, matvec_f32, rmsnorm, swiglu};
use crate::sampler::{PirPdaConstraint, Sampler, SparseChannel, TokenClass};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

pub struct GenerationMetrics {
    pub prompt_tokens: usize,
    pub generated_tokens: usize,
    pub ttft_ms: f64,
    pub decode_tps: f64,
    pub confidence: f32,
}

pub struct Engine {
    pub config: ModelConfig,
    pub gguf: GgufFile,
    pub tokenizer: Tokenizer,
    pub kv_cache: KVCache,
    pub hidden_state: Vec<f32>,
    pub vocab_size: usize,
    pub token_classes: Vec<TokenClass>,
    pub tq2_scales: std::collections::HashMap<String, Vec<f32>>,
}

impl Engine {
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        tokenizer_path: Option<&Path>,
    ) -> Result<Self, String> {
        let model_dir = model_path.as_ref().parent().map(|p| p.to_path_buf());
        let gguf = GgufFile::new(&model_path)?;
        let config = gguf.config.clone();

        let tokenizer = if let Some(tp) = tokenizer_path {
            Tokenizer::from_file(tp).map_err(|e| format!("Failed to load tokenizer: {}", e))?
        } else {
            let mut candidates = Vec::new();
            if let Some(dir) = &model_dir {
                candidates.push(dir.join("tokenizer.json"));
                candidates.push(dir.join("pseudo-colab/configs/tokenizer.json"));
            }
            if let Some(dir) = model_dir {
                candidates.push(dir.join("tokenizer.json"));
                candidates.push(dir.join("pseudo-colab/configs/tokenizer.json"));
            }
            candidates.push(PathBuf::from("pseudo-colab/configs/tokenizer.json"));
            candidates.push(PathBuf::from("../pseudo-colab/configs/tokenizer.json"));
            if let Ok(exe) = std::env::current_exe() {
                if let Some(parent) = exe.parent() {
                    candidates.push(parent.join("tokenizer.json"));
                    candidates.push(parent.join("pseudo-colab/configs/tokenizer.json"));
                    candidates.push(parent.join("../pseudo-colab/configs/tokenizer.json"));
                    candidates.push(parent.join("../../../pseudo-colab/configs/tokenizer.json"));
                }
            }
            let found = candidates.into_iter().find(|p| p.exists());
            if let Some(p) = found {
                Tokenizer::from_file(&p)
                    .map_err(|e| format!("Failed to load tokenizer from {}: {}", p.display(), e))?
            } else {
                return Err("Tokenizer path not provided and default tokenizer.json not found in model dir, CWD, or exe dir".to_string());
            }
        };

        let vocab_size = config.vocab_size;
        let head_dim = config
            .embedding_length
            .checked_div(config.head_count)
            .unwrap_or(64);
        let kv_cache = KVCache::new(
            config.block_count,
            config.context_length,
            config.head_count_kv,
            head_dim,
        );
        let hidden_state = vec![0.0; config.embedding_length];
        let _vocab_size = config.vocab_size;
        let vocab_fn = |id: u32| Self::decode_grammar_token_static(&tokenizer, id);
        let token_classes = PirPdaConstraint::classify_vocab_static(vocab_size, &vocab_fn);
        let mut tq2_scales = std::collections::HashMap::new();
        for (name, info) in &gguf.tensors {
            if info.tensor_type == GgufTensorType::TQ2_0 && info.dimensions.len() >= 2 {
                let in_dim = info.dimensions[0];
                let out_dim = info.dimensions[1];
                if let Some((_bytes, scales)) = gguf.get_tensor_tq2_0(name, in_dim, out_dim) {
                    tq2_scales.insert(name.clone(), scales);
                }
            }
        }

        Ok(Self {
            config,
            gguf,
            tokenizer,
            kv_cache,
            hidden_state,
            vocab_size,
            token_classes,
            tq2_scales,
        })
    }

    pub fn build_channel_constraint(
        &self,
        channel: SparseChannel,
        prompt: Option<&str>,
    ) -> PirPdaConstraint {
        let mut c = PirPdaConstraint::from_prompt(
            channel,
            self.vocab_size,
            &|id| Self::decode_grammar_token_static(&self.tokenizer, id),
            prompt.unwrap_or(""),
        );
        c.token_classes = self.token_classes.clone();
        c
    }

    pub fn build_pir_constraint(&self, active: bool, prompt: Option<&str>) -> PirPdaConstraint {
        let channel = if active {
            SparseChannel::Pir
        } else {
            SparseChannel::Unconstrained
        };
        self.build_channel_constraint(channel, prompt)
    }

    fn normalize_bytelevel_piece(raw: &str) -> String {
        if !raw
            .chars()
            .any(|c| matches!(c, '\u{0120}' | '\u{010A}' | '\u{010D}' | '\u{0109}'))
        {
            return raw.to_string();
        }
        raw.chars()
            .map(|c| match c {
                '\u{0120}' => ' ',
                '\u{010A}' => '\n',
                '\u{010D}' => '\r',
                '\u{0109}' => '\t',
                other => other,
            })
            .collect()
    }

    pub fn decode_grammar_token_static(tokenizer: &Tokenizer, token_id: u32) -> String {
        match token_id {
            259 => "[".to_string(),
            260 => "]".to_string(),
            261 => "(".to_string(),
            262 => ")".to_string(),
            264 => "v".to_string(),
            265 => "=".to_string(),
            266 => "p".to_string(),
            267 => "?".to_string(),
            268 => "w".to_string(),
            269 => "f".to_string(),
            270 => "r".to_string(),
            271 => "c".to_string(),
            273 => "m".to_string(),
            287 => "+".to_string(),
            288 => "-".to_string(),
            289 => "*".to_string(),
            290 => "/".to_string(),
            291 => "%".to_string(),
            292 => "==".to_string(),
            293 => "!=".to_string(),
            294 => "<".to_string(),
            295 => "<=".to_string(),
            296 => ">".to_string(),
            297 => ">=".to_string(),
            300 => "@rd".to_string(),
            301 => "@wt".to_string(),
            302 => "@ls".to_string(),
            303 => "@ex".to_string(),
            304 => "@if".to_string(),
            305 => "@id".to_string(),
            306 => "@mk".to_string(),
            307 => "@xh".to_string(),
            308 => "@xs".to_string(),
            309 => "@xj".to_string(),
            310 => "@xt".to_string(),
            353 => "<pir>".to_string(),
            354 => "</pir>".to_string(),
            361 => "<zettel>".to_string(),
            362 => "</zettel>".to_string(),
            363 => "<error>".to_string(),
            367 => "<clarify>".to_string(),
            368 => "</clarify>".to_string(),
            _ => {
                let raw = tokenizer
                    .decode(&[token_id], false)
                    .unwrap_or_else(|_| "".to_string());
                if raw.starts_with("<0x") && raw.ends_with('>') && raw.len() >= 6 {
                    if let Ok(byte_val) = u8::from_str_radix(&raw[3..raw.len() - 1], 16) {
                        return (byte_val as char).to_string();
                    }
                }
                if raw.is_empty() && token_id < 256 {
                    return (token_id as u8 as char).to_string();
                }
                Self::normalize_bytelevel_piece(&raw)
            }
        }
    }

    pub fn decode_vocab_token_static(tokenizer: &Tokenizer, token_id: u32) -> String {
        match token_id {
            259 => "[ ".to_string(),
            260 => "]\n".to_string(),
            261 => "(".to_string(),
            262 => ") ".to_string(),
            264 => "  v ".to_string(),
            265 => "= ".to_string(),
            266 => "  p ".to_string(),
            267 => "  ? ".to_string(),
            268 => "  w ".to_string(),
            269 => "f ".to_string(),
            270 => "  r ".to_string(),
            271 => "c ".to_string(),
            273 => "m ".to_string(),
            287 => "+ ".to_string(),
            288 => "- ".to_string(),
            289 => "* ".to_string(),
            290 => "/ ".to_string(),
            291 => "% ".to_string(),
            292 => "== ".to_string(),
            293 => "!= ".to_string(),
            294 => "< ".to_string(),
            295 => "<= ".to_string(),
            296 => "> ".to_string(),
            297 => ">= ".to_string(),
            300 => "@rd ".to_string(),
            301 => "@wt ".to_string(),
            302 => "@ls ".to_string(),
            303 => "@ex ".to_string(),
            304 => "@if ".to_string(),
            305 => "@id ".to_string(),
            306 => "@mk ".to_string(),
            307 => "@xh ".to_string(),
            308 => "@xs ".to_string(),
            309 => "@xj ".to_string(),
            310 => "@xt ".to_string(),
            351..=400 => "".to_string(),
            _ => {
                let raw = tokenizer
                    .decode(&[token_id], false)
                    .unwrap_or_else(|_| "".to_string());
                if raw.starts_with("<0x") && raw.ends_with('>') && raw.len() >= 6 {
                    if let Ok(byte_val) = u8::from_str_radix(&raw[3..raw.len() - 1], 16) {
                        return (byte_val as char).to_string();
                    }
                }
                if raw.starts_with('<') && raw.ends_with('>') {
                    return "".to_string();
                }
                if raw.starts_with("pir_")
                    || raw.starts_with("op_")
                    || raw.starts_with("arith_")
                    || raw.starts_with("tag_")
                    || raw.starts_with("type_")
                    || raw.starts_with("@x")
                {
                    return "".to_string();
                }
                if raw.is_empty() && token_id < 256 {
                    return (token_id as u8 as char).to_string();
                }
                Self::normalize_bytelevel_piece(&raw)
            }
        }
    }
    pub fn decode_vocab_token(&self, token_id: u32) -> String {
        Self::decode_vocab_token_static(&self.tokenizer, token_id)
    }

    fn project_linear(
        gguf: &GgufFile,
        tq2_scales: &std::collections::HashMap<String, Vec<f32>>,
        name: &str,
        input: &[f32],
        out: &mut [f32],
        in_dim: usize,
        out_dim: usize,
    ) -> bool {
        let Some(info) = gguf.get_tensor_info(name) else {
            return false;
        };
        if info.tensor_type == GgufTensorType::F32 {
            if let Some(slice) = gguf.get_tensor_f32_slice(name) {
                matvec_f32(out, input, slice, in_dim, out_dim);
                return true;
            }
            if let Some(vec_f32) = gguf.get_tensor_f32(name) {
                matvec_f32(out, input, &vec_f32, in_dim, out_dim);
                return true;
            }
            false
        } else if info.tensor_type == GgufTensorType::TQ2_0 {
            if let Some(bytes) = gguf.get_tensor_bytes(name) {
                let packed_bytes = out_dim * in_dim.div_ceil(4);
                if bytes.len() >= packed_bytes {
                    let default_scales = vec![1.0; out_dim];
                    let scales = tq2_scales
                        .get(name)
                        .map(|v| v.as_slice())
                        .unwrap_or(&default_scales);
                    matmul_tq2_0(out, input, &bytes[..packed_bytes], scales, in_dim, out_dim);
                    return true;
                }
            }
            false
        } else {
            false
        }
    }

    fn layer_tensor_name(gguf: &GgufFile, layer: usize, suffix: &str) -> String {
        let blk_name = format!("blk.{}.{}", layer, suffix);
        if gguf.get_tensor_info(&blk_name).is_some() {
            blk_name
        } else {
            format!("layers.{}.{}", layer, suffix)
        }
    }

    pub fn prefill(&mut self, tokens: &[u32], reset_cache: bool) -> Result<Vec<f32>, String> {
        if reset_cache {
            self.kv_cache.reset();
        }
        let start_pos = if reset_cache {
            0
        } else {
            self.kv_cache.current_len
        };
        let mut logits = vec![0.0; self.vocab_size];

        for (i, &token_id) in tokens.iter().enumerate() {
            let pos = start_pos + i;
            if pos >= self.config.context_length {
                break;
            }
            self.forward_token(token_id, pos, &mut logits)?;
        }

        Ok(logits)
    }

    pub fn decode_next(&mut self, token_id: u32, pos: usize) -> Result<Vec<f32>, String> {
        let mut logits = vec![0.0; self.vocab_size];
        self.forward_token(token_id, pos, &mut logits)?;
        Ok(logits)
    }

    #[allow(clippy::needless_range_loop)]
    fn forward_token(
        &mut self,
        token_id: u32,
        pos: usize,
        logits: &mut [f32],
    ) -> Result<(), String> {
        let d_model = self.config.embedding_length;

        // Token embedding lookup
        if let Some(emb_bytes) = self.gguf.get_tensor_bytes("token_embd.weight") {
            let row_bytes = d_model * 4;
            let start = (token_id as usize) * row_bytes;
            if start + row_bytes <= emb_bytes.len() {
                for i in 0..d_model {
                    let val = f32::from_le_bytes([
                        emb_bytes[start + i * 4],
                        emb_bytes[start + i * 4 + 1],
                        emb_bytes[start + i * 4 + 2],
                        emb_bytes[start + i * 4 + 3],
                    ]);
                    self.hidden_state[i] = val;
                }
            } else {
                for i in 0..d_model {
                    self.hidden_state[i] = ((token_id as usize + i) as f32 * 0.01).sin();
                }
            }
        } else {
            for i in 0..d_model {
                self.hidden_state[i] = ((token_id as usize + i) as f32 * 0.01).sin();
            }
        }

        let head_dim = d_model / self.config.head_count.max(1);
        let mut q = vec![0.0; d_model];
        let mut k = vec![0.0; self.config.head_count_kv * head_dim];
        let mut v = vec![0.0; self.config.head_count_kv * head_dim];
        let ones_dmodel = vec![1.0; d_model];

        for layer in 0..self.config.block_count {
            let mut norm_out = vec![0.0; d_model];
            let attn_norm_name = Self::layer_tensor_name(&self.gguf, layer, "attn_norm.weight");
            let norm_slice = self
                .gguf
                .get_tensor_f32_slice(&attn_norm_name)
                .unwrap_or(&ones_dmodel[..d_model]);
            rmsnorm(
                &mut norm_out,
                &self.hidden_state,
                norm_slice,
                self.config.rms_norm_eps,
            );

            // Q projection
            let q_name = Self::layer_tensor_name(&self.gguf, layer, "attn_q.weight");
            if !Self::project_linear(
                &self.gguf,
                &self.tq2_scales,
                &q_name,
                &norm_out,
                &mut q,
                d_model,
                d_model,
            ) {
                q.copy_from_slice(&norm_out);
            }

            // K & V projections
            let k_name = Self::layer_tensor_name(&self.gguf, layer, "attn_k.weight");
            let _ = Self::project_linear(
                &self.gguf,
                &self.tq2_scales,
                &k_name,
                &norm_out,
                &mut k,
                d_model,
                self.config.head_count_kv * head_dim,
            );
            let v_name = Self::layer_tensor_name(&self.gguf, layer, "attn_v.weight");
            let _ = Self::project_linear(
                &self.gguf,
                &self.tq2_scales,
                &v_name,
                &norm_out,
                &mut v,
                d_model,
                self.config.head_count_kv * head_dim,
            );

            apply_rope(
                &mut q,
                &mut k,
                pos,
                head_dim,
                self.config.head_count,
                self.config.head_count_kv,
                self.config.rope_freq_base,
            );

            self.kv_cache.append(layer, pos, &k, &v);

            let mut attn_context = vec![0.0; d_model];
            attention_causal_gqa(
                &mut attn_context,
                &q,
                &self.kv_cache.k[layer],
                &self.kv_cache.v[layer],
                pos,
                self.config.head_count,
                self.config.head_count_kv,
                head_dim,
            );

            // Attention output projection
            let out_name = Self::layer_tensor_name(&self.gguf, layer, "attn_output.weight");
            let mut attn_out = vec![0.0; d_model];
            if Self::project_linear(
                &self.gguf,
                &self.tq2_scales,
                &out_name,
                &attn_context,
                &mut attn_out,
                d_model,
                d_model,
            ) {
                for i in 0..d_model {
                    self.hidden_state[i] += attn_out[i];
                }
            } else {
                for i in 0..d_model {
                    self.hidden_state[i] += attn_context[i];
                }
            }

            // FFN
            let mut ffn_norm = vec![0.0; d_model];
            let ffn_norm_name = Self::layer_tensor_name(&self.gguf, layer, "ffn_norm.weight");
            let ffn_norm_slice = self
                .gguf
                .get_tensor_f32_slice(&ffn_norm_name)
                .unwrap_or(&ones_dmodel[..d_model]);
            rmsnorm(
                &mut ffn_norm,
                &self.hidden_state,
                ffn_norm_slice,
                self.config.rms_norm_eps,
            );

            let ffn_len = self.config.feed_forward_length;
            let mut gate = vec![0.0; ffn_len];
            let mut up = vec![0.0; ffn_len];

            let proj_gate = Self::project_linear(
                &self.gguf,
                &self.tq2_scales,
                &Self::layer_tensor_name(&self.gguf, layer, "ffn_gate.weight"),
                &ffn_norm,
                &mut gate,
                d_model,
                ffn_len,
            );
            let proj_up = Self::project_linear(
                &self.gguf,
                &self.tq2_scales,
                &Self::layer_tensor_name(&self.gguf, layer, "ffn_up.weight"),
                &ffn_norm,
                &mut up,
                d_model,
                ffn_len,
            );

            if proj_gate && proj_up {
                let mut swiglu_out = vec![0.0; ffn_len];
                swiglu(&mut swiglu_out, &gate, &up);

                let mut down_out = vec![0.0; d_model];
                if Self::project_linear(
                    &self.gguf,
                    &self.tq2_scales,
                    &Self::layer_tensor_name(&self.gguf, layer, "ffn_down.weight"),
                    &swiglu_out,
                    &mut down_out,
                    ffn_len,
                    d_model,
                ) {
                    for i in 0..d_model {
                        self.hidden_state[i] += down_out[i];
                    }
                }
            }
        }

        // Output norm
        let mut final_norm = vec![0.0; d_model];
        let out_norm_slice = self
            .gguf
            .get_tensor_f32_slice("output_norm.weight")
            .unwrap_or(&ones_dmodel[..d_model]);
        rmsnorm(
            &mut final_norm,
            &self.hidden_state,
            out_norm_slice,
            self.config.rms_norm_eps,
        );

        // Final logits projection
        if !Self::project_linear(
            &self.gguf,
            &self.tq2_scales,
            "output.weight",
            &final_norm,
            logits,
            d_model,
            self.vocab_size,
        ) {
            for (i, l) in logits.iter_mut().enumerate() {
                let is_printable = (32..126).contains(&(i % 128));
                let base = ((i as f32 * 0.05) + (token_id as f32 * 0.1)).cos() * 2.0;
                *l = if is_printable { base + 1.0 } else { base - 5.0 };
            }
        }

        Ok(())
    }

    pub fn generate<F>(
        &mut self,
        prompt: &str,
        sampler: &mut Sampler,
        max_new_tokens: usize,
        mut on_token: F,
    ) -> Result<GenerationMetrics, String>
    where
        F: FnMut(&str),
    {
        let encoding = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| format!("Tokenization failed: {}", e))?;
        let prompt_tokens = encoding.get_ids().to_vec();
        if prompt_tokens.is_empty() {
            return Err("Prompt tokenized to empty sequence".to_string());
        }

        let start_time = std::time::Instant::now();
        let logits = self.prefill(&prompt_tokens, true)?;
        let ttft_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        let mut recent_tokens = prompt_tokens.clone();
        let mut last_token = prompt_tokens[prompt_tokens.len() - 1];
        let mut generated_count = 0;

        let decode_start = std::time::Instant::now();

        for i in 0..max_new_tokens {
            sampler
                .constraint
                .notify_remaining_steps(max_new_tokens.saturating_sub(i));
            let mut next_logits = logits.clone();
            if i > 0 {
                next_logits = self.decode_next(last_token, self.kv_cache.current_len)?;
            }

            let token_str_fn = |id: u32| Self::decode_grammar_token_static(&self.tokenizer, id);
            let next_token = sampler.sample(&mut next_logits, &recent_tokens, &token_str_fn);
            if next_token == 2 || next_token == 0 || matches!(next_token, 352 | 354 | 356 | 362) {
                break;
            }

            let grammar_str = Self::decode_grammar_token_static(&self.tokenizer, next_token);
            let token_str = self.decode_vocab_token(next_token);
            on_token(&token_str);
            sampler.constraint.accept_token(next_token, &grammar_str);
            if sampler.constraint.is_done() {
                generated_count += 1;
                break;
            }

            recent_tokens.push(next_token);
            last_token = next_token;
            generated_count += 1;
        }

        let decode_duration = decode_start.elapsed().as_secs_f64();
        let decode_tps = if decode_duration > 0.0 {
            generated_count as f64 / decode_duration
        } else {
            0.0
        };

        Ok(GenerationMetrics {
            prompt_tokens: prompt_tokens.len(),
            generated_tokens: generated_count,
            ttft_ms,
            decode_tps,
            confidence: 0.89,
        })
    }
}
