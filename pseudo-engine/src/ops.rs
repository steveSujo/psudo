use rayon::prelude::*;

pub fn rmsnorm(out: &mut [f32], x: &[f32], weight: &[f32], eps: f32) {
    let n = x.len();
    if n == 0 {
        return;
    }
    let sum_sq: f32 = x.iter().map(|&v| v * v).sum();
    let mean_sq = sum_sq / (n as f32);
    let scale = 1.0 / (mean_sq + eps).sqrt();

    for i in 0..n {
        let w = if i < weight.len() { weight[i] } else { 1.0 };
        out[i] = x[i] * scale * w;
    }
}

pub fn rmsnorm_add_residual(
    out: &mut [f32],
    residual: &mut [f32],
    x: &[f32],
    weight: &[f32],
    eps: f32,
) {
    let n = x.len().min(out.len()).min(residual.len());
    if n == 0 {
        return;
    }
    for i in 0..n {
        residual[i] += x[i];
    }
    let sum_sq: f32 = residual[..n].iter().map(|&v| v * v).sum();
    let mean_sq = sum_sq / (n as f32);
    let scale = 1.0 / (mean_sq + eps).sqrt();

    for i in 0..n {
        let w = if i < weight.len() { weight[i] } else { 1.0 };
        out[i] = residual[i] * scale * w;
    }
}

pub fn apply_rope(
    q: &mut [f32],
    k: &mut [f32],
    pos: usize,
    head_dim: usize,
    n_heads: usize,
    n_kv_heads: usize,
    theta: f32,
) {
    let half = head_dim / 2;
    for h in 0..n_heads {
        let offset = h * head_dim;
        for i in 0..half {
            let freq = 1.0 / theta.powf((2 * i) as f32 / head_dim as f32);
            let val = (pos as f32) * freq;
            let cos = val.cos();
            let sin = val.sin();

            let idx0 = offset + i;
            let idx1 = offset + i + half;
            if idx1 < q.len() {
                let x0 = q[idx0];
                let x1 = q[idx1];
                q[idx0] = x0 * cos - x1 * sin;
                q[idx1] = x0 * sin + x1 * cos;
            }
        }
    }

    for h in 0..n_kv_heads {
        let offset = h * head_dim;
        for i in 0..half {
            let freq = 1.0 / theta.powf((2 * i) as f32 / head_dim as f32);
            let val = (pos as f32) * freq;
            let cos = val.cos();
            let sin = val.sin();

            let idx0 = offset + i;
            let idx1 = offset + i + half;
            if idx1 < k.len() {
                let x0 = k[idx0];
                let x1 = k[idx1];
                k[idx0] = x0 * cos - x1 * sin;
                k[idx1] = x0 * sin + x1 * cos;
            }
        }
    }
}

pub fn swiglu(out: &mut [f32], gate: &[f32], up: &[f32]) {
    let n = out.len().min(gate.len()).min(up.len());
    let chunks = n / 4;
    for c in 0..chunks {
        let base = c * 4;
        let g0 = gate[base];
        let g1 = gate[base + 1];
        let g2 = gate[base + 2];
        let g3 = gate[base + 3];

        out[base] = (g0 / (1.0 + (-g0).exp())) * up[base];
        out[base + 1] = (g1 / (1.0 + (-g1).exp())) * up[base + 1];
        out[base + 2] = (g2 / (1.0 + (-g2).exp())) * up[base + 2];
        out[base + 3] = (g3 / (1.0 + (-g3).exp())) * up[base + 3];
    }
    for i in (chunks * 4)..n {
        let g = gate[i];
        let silu = g / (1.0 + (-g).exp());
        out[i] = silu * up[i];
    }
}

pub fn softmax(logits: &mut [f32]) {
    if logits.is_empty() {
        return;
    }
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    if max.is_infinite() && max.is_sign_negative() {
        let uniform = 1.0 / logits.len() as f32;
        logits.fill(uniform);
        return;
    }
    let mut sum = 0.0;
    for l in logits.iter_mut() {
        *l = (*l - max).exp();
        sum += *l;
    }
    if sum > 0.0 {
        for l in logits.iter_mut() {
            *l /= sum;
        }
    } else {
        let uniform = 1.0 / logits.len() as f32;
        logits.fill(uniform);
    }
}

pub fn matmul_f32(out: &mut [f32], a: &[f32], b: &[f32], _m: usize, n: usize, k: usize) {
    out.par_iter_mut().enumerate().for_each(|(idx, val)| {
        let row = idx / n;
        let col = idx % n;
        let mut sum = 0.0;
        let a_row_offset = row * k;
        let b_col_offset = col * k;
        for s in 0..k {
            if a_row_offset + s < a.len() && b_col_offset + s < b.len() {
                sum += a[a_row_offset + s] * b[b_col_offset + s];
            }
        }
        *val = sum;
    });
}

pub fn matvec_f32(out: &mut [f32], x: &[f32], weights: &[f32], in_dim: usize, _out_dim: usize) {
    if weights.is_empty() {
        out.fill(0.0);
        return;
    }
    let compute = |(out_idx, val): (usize, &mut f32)| {
        let row_start = out_idx * in_dim;
        let mut sum = 0.0f32;
        for col in 0..in_dim {
            if col < x.len() && row_start + col < weights.len() {
                sum += weights[row_start + col] * x[col];
            }
        }
        *val = sum;
    };
    // Below 32k ops, Rayon thread-pool dispatch costs more than the math itself.
    if out.len() * in_dim < 32_768 {
        for item in out.iter_mut().enumerate() {
            compute(item);
        }
    } else {
        out.par_iter_mut().enumerate().for_each(compute);
    }
}

// Packing four ternary weights {-1, 0, 1} into one byte cuts the model down to 2 bits per weight—
// that footprint reduction is what lets a ~100M model fit and run fast on my machine.
const TQ2_MAP: [f32; 4] = [0.0, 1.0, -1.0, 0.0];

pub fn matmul_tq2_0(
    out: &mut [f32],
    x: &[f32],
    weights: &[u8],
    scales: &[f32],
    in_dim: usize,
    _out_dim: usize,
) {
    if weights.is_empty() || in_dim == 0 {
        out.fill(0.0);
        return;
    }
    let full_bytes = in_dim / 4;
    let row_bytes = in_dim.div_ceil(4);

    let compute = |(out_idx, val): (usize, &mut f32)| {
        let row_offset = out_idx * row_bytes;
        if row_offset >= weights.len() {
            *val = 0.0;
            return;
        }
        let scale = if out_idx < scales.len() {
            scales[out_idx]
        } else {
            1.0
        };
        let mut sum = 0.0f32;

        let num_bytes = full_bytes
            .min(weights.len().saturating_sub(row_offset))
            .min(x.len() / 4);
        for b_idx in 0..num_bytes {
            let b = weights[row_offset + b_idx];
            let col = b_idx * 4;
            let w0 = TQ2_MAP[(b & 0x03) as usize];
            let w1 = TQ2_MAP[((b >> 2) & 0x03) as usize];
            let w2 = TQ2_MAP[((b >> 4) & 0x03) as usize];
            let w3 = TQ2_MAP[((b >> 6) & 0x03) as usize];
            sum += w0 * x[col] + w1 * x[col + 1] + w2 * x[col + 2] + w3 * x[col + 3];
        }

        let rem_start = num_bytes * 4;
        for col in rem_start..in_dim {
            if col >= x.len() {
                break;
            }
            let byte_idx = row_offset + (col / 4);
            if byte_idx < weights.len() {
                let b = weights[byte_idx];
                let shift = (col % 4) * 2;
                let twobit = (b >> shift) & 0x03;
                sum += TQ2_MAP[twobit as usize] * x[col];
            }
        }

        *val = sum * scale;
    };

    // Below 32k ops, Rayon thread-pool dispatch costs more than the math itself.
    if out.len() * in_dim < 32_768 {
        for item in out.iter_mut().enumerate() {
            compute(item);
        }
    } else {
        out.par_iter_mut().enumerate().for_each(compute);
    }
}

pub fn fwht(data: &mut [f32]) {
    let n = data.len();
    if n <= 1 {
        return;
    }
    if n.is_power_of_two() {
        let mut len = 1;
        while len < n {
            for i in (0..n).step_by(len * 2) {
                for j in 0..len {
                    let u = data[i + j];
                    let v = data[i + len + j];
                    data[i + j] = u + v;
                    data[i + len + j] = u - v;
                }
            }
            len *= 2;
        }
        let norm = 1.0 / (n as f32).sqrt();
        for x in data.iter_mut() {
            *x *= norm;
        }
    } else {
        let next_pow2 = n.next_power_of_two();
        let mut padded = vec![0.0f32; next_pow2];
        padded[..n].copy_from_slice(data);
        fwht(&mut padded);
        data.copy_from_slice(&padded[..n]);
    }
}

#[allow(clippy::needless_range_loop)]
pub fn dequantize_turboquant_h_row(out: &mut [f32], packed: &[u8], scale: f32) {
    let n = out.len();
    if n == 0 {
        return;
    }
    let full_bytes = n / 4;
    let num_bytes = full_bytes.min(packed.len());
    for b_idx in 0..num_bytes {
        let b = packed[b_idx];
        let col = b_idx * 4;
        out[col] = TQ2_MAP[(b & 0x03) as usize];
        out[col + 1] = TQ2_MAP[((b >> 2) & 0x03) as usize];
        out[col + 2] = TQ2_MAP[((b >> 4) & 0x03) as usize];
        out[col + 3] = TQ2_MAP[((b >> 6) & 0x03) as usize];
    }
    for col in (num_bytes * 4)..n {
        let byte_idx = col / 4;
        if byte_idx < packed.len() {
            let b = packed[byte_idx];
            let shift = (col % 4) * 2;
            out[col] = TQ2_MAP[((b >> shift) & 0x03) as usize];
        } else {
            out[col] = 0.0;
        }
    }
    fwht(out);
    for v in out.iter_mut() {
        *v *= scale;
    }
}

#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
pub fn attention_causal_gqa(
    out: &mut [f32],
    q: &[f32],
    k_cache: &[f32],
    v_cache: &[f32],
    pos: usize,
    n_heads: usize,
    n_kv_heads: usize,
    head_dim: usize,
) {
    if n_heads == 0 || n_kv_heads == 0 || head_dim == 0 {
        return;
    }
    let scale = 1.0 / (head_dim as f32).sqrt();
    let num_tokens = pos + 1;
    let heads_per_kv = (n_heads / n_kv_heads).max(1);

    for h in 0..n_heads {
        let kv_h = h / heads_per_kv;
        let q_offset = h * head_dim;

        let mut scores = vec![0.0f32; num_tokens];
        for t in 0..num_tokens {
            let k_offset = (t * n_kv_heads + kv_h) * head_dim;
            let mut dot = 0.0f32;
            for d in 0..head_dim {
                if q_offset + d < q.len() && k_offset + d < k_cache.len() {
                    dot += q[q_offset + d] * k_cache[k_offset + d];
                }
            }
            scores[t] = dot * scale;
        }

        softmax(&mut scores);

        let out_offset = h * head_dim;
        for d in 0..head_dim {
            let mut val_sum = 0.0f32;
            for t in 0..num_tokens {
                let v_offset = (t * n_kv_heads + kv_h) * head_dim;
                if v_offset + d < v_cache.len() {
                    val_sum += scores[t] * v_cache[v_offset + d];
                }
            }
            if out_offset + d < out.len() {
                out[out_offset + d] = val_sum;
            }
        }
    }
}
