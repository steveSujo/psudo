// Quantizing KV tensors to INT8 trades a sliver of precision for half the cache footprint—
// essential when my context window shares tight memory with the model weights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KVCachePrecision {
    FP32,
    INT8,
}

#[derive(Clone)]
pub struct KVCache {
    pub n_layers: usize,
    pub max_seq_len: usize,
    pub n_kv_heads: usize,
    pub head_dim: usize,
    pub current_len: usize,
    pub precision: KVCachePrecision,
    pub k: Vec<Vec<f32>>,
    pub v: Vec<Vec<f32>>,
    pub k_int8: Vec<Vec<i8>>,
    pub v_int8: Vec<Vec<i8>>,
    pub k_scales: Vec<Vec<f32>>,
    pub v_scales: Vec<Vec<f32>>,
}

impl KVCache {
    pub fn new(n_layers: usize, max_seq_len: usize, n_kv_heads: usize, head_dim: usize) -> Self {
        Self::new_with_precision(
            n_layers,
            max_seq_len,
            n_kv_heads,
            head_dim,
            KVCachePrecision::FP32,
        )
    }

    pub fn new_with_precision(
        n_layers: usize,
        max_seq_len: usize,
        n_kv_heads: usize,
        head_dim: usize,
        precision: KVCachePrecision,
    ) -> Self {
        let layer_size = max_seq_len * n_kv_heads * head_dim;
        let scale_size = max_seq_len * n_kv_heads;

        let (k_int8, v_int8, k_scales, v_scales) = if precision == KVCachePrecision::INT8 {
            (
                vec![vec![0i8; layer_size]; n_layers],
                vec![vec![0i8; layer_size]; n_layers],
                vec![vec![1.0f32; scale_size]; n_layers],
                vec![vec![1.0f32; scale_size]; n_layers],
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };

        Self {
            n_layers,
            max_seq_len,
            n_kv_heads,
            head_dim,
            current_len: 0,
            precision,
            k: vec![vec![0.0; layer_size]; n_layers],
            v: vec![vec![0.0; layer_size]; n_layers],
            k_int8,
            v_int8,
            k_scales,
            v_scales,
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub fn append(&mut self, layer: usize, pos: usize, k_data: &[f32], v_data: &[f32]) {
        if layer >= self.n_layers || pos >= self.max_seq_len {
            return;
        }
        let head_size = self.n_kv_heads * self.head_dim;
        let start = pos * head_size;
        let end = start + head_size;

        if end <= self.k[layer].len() && k_data.len() >= head_size {
            if self.precision == KVCachePrecision::INT8 {
                for h in 0..self.n_kv_heads {
                    let h_start = start + h * self.head_dim;
                    let slice_k = &k_data[h * self.head_dim..(h + 1) * self.head_dim];
                    let max_k = slice_k.iter().fold(0.0f32, |acc, &v| acc.max(v.abs()));
                    let scale_k = if max_k > 0.0 { max_k / 127.0 } else { 1.0 };

                    let scale_idx = pos * self.n_kv_heads + h;
                    if scale_idx < self.k_scales[layer].len() {
                        self.k_scales[layer][scale_idx] = scale_k;
                    }
                    for d in 0..self.head_dim {
                        let q_val = (slice_k[d] / scale_k).round().clamp(-127.0, 127.0) as i8;
                        self.k_int8[layer][h_start + d] = q_val;
                        self.k[layer][h_start + d] = (q_val as f32) * scale_k;
                    }
                }
            } else {
                self.k[layer][start..end].copy_from_slice(&k_data[..head_size]);
            }
        }

        if end <= self.v[layer].len() && v_data.len() >= head_size {
            if self.precision == KVCachePrecision::INT8 {
                for h in 0..self.n_kv_heads {
                    let h_start = start + h * self.head_dim;
                    let slice_v = &v_data[h * self.head_dim..(h + 1) * self.head_dim];
                    let max_v = slice_v.iter().fold(0.0f32, |acc, &v| acc.max(v.abs()));
                    let scale_v = if max_v > 0.0 { max_v / 127.0 } else { 1.0 };

                    let scale_idx = pos * self.n_kv_heads + h;
                    if scale_idx < self.v_scales[layer].len() {
                        self.v_scales[layer][scale_idx] = scale_v;
                    }
                    for d in 0..self.head_dim {
                        let q_val = (slice_v[d] / scale_v).round().clamp(-127.0, 127.0) as i8;
                        self.v_int8[layer][h_start + d] = q_val;
                        self.v[layer][h_start + d] = (q_val as f32) * scale_v;
                    }
                }
            } else {
                self.v[layer][start..end].copy_from_slice(&v_data[..head_size]);
            }
        }

        if pos + 1 > self.current_len {
            self.current_len = pos + 1;
        }
    }
    // Forking the KV cache lets me branch speculative or auto-heal runs without re-evaluating the pinned prefix.
    pub fn fork_sequence(&self, up_to_pos: usize) -> Self {
        let mut forked = Self::new_with_precision(
            self.n_layers,
            self.max_seq_len,
            self.n_kv_heads,
            self.head_dim,
            self.precision,
        );
        let head_size = self.n_kv_heads * self.head_dim;
        let copy_len = up_to_pos * head_size;
        let scale_copy_len = up_to_pos * self.n_kv_heads;

        for layer in 0..self.n_layers {
            let max_avail = copy_len.min(self.k[layer].len());
            forked.k[layer][..max_avail].copy_from_slice(&self.k[layer][..max_avail]);
            forked.v[layer][..max_avail].copy_from_slice(&self.v[layer][..max_avail]);

            if self.precision == KVCachePrecision::INT8 {
                forked.k_int8[layer][..max_avail].copy_from_slice(&self.k_int8[layer][..max_avail]);
                forked.v_int8[layer][..max_avail].copy_from_slice(&self.v_int8[layer][..max_avail]);
                let max_scale = scale_copy_len.min(self.k_scales[layer].len());
                forked.k_scales[layer][..max_scale]
                    .copy_from_slice(&self.k_scales[layer][..max_scale]);
                forked.v_scales[layer][..max_scale]
                    .copy_from_slice(&self.v_scales[layer][..max_scale]);
            }
        }
        forked.current_len = up_to_pos;
        forked
    }

    pub fn reset(&mut self) {
        self.current_len = 0;
    }
}
