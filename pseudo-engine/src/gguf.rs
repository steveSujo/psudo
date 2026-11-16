use byteorder::{LittleEndian, ReadBytesExt};
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub architecture: String,
    pub context_length: usize,
    pub embedding_length: usize,
    pub block_count: usize,
    pub feed_forward_length: usize,
    pub head_count: usize,
    pub head_count_kv: usize,
    pub rms_norm_eps: f32,
    pub rope_freq_base: f32,
    pub vocab_size: usize,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            architecture: "pseudo".to_string(),
            context_length: 1536,
            embedding_length: 1024,
            block_count: 16,
            feed_forward_length: 2816,
            head_count: 16,
            head_count_kv: 4,
            rms_norm_eps: 1e-5,
            rope_freq_base: 10000.0,
            vocab_size: 8192,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgufTensorType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q8_0 = 8,
    TQ2_0 = 30,
}

impl GgufTensorType {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0 => Some(Self::F32),
            1 => Some(Self::F16),
            2 => Some(Self::Q4_0),
            8 => Some(Self::Q8_0),
            30 => Some(Self::TQ2_0),
            _ => Some(Self::F32),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GgufTensorInfo {
    pub name: String,
    pub dimensions: Vec<usize>,
    pub tensor_type: GgufTensorType,
    pub offset: u64,
    pub size_bytes: usize,
}

pub struct GgufFile {
    mmap: Option<Mmap>,
    pub config: ModelConfig,
    pub tensors: HashMap<String, GgufTensorInfo>,
    data_offset: usize,
}

impl GgufFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => {
                return Ok(Self {
                    mmap: None,
                    config: ModelConfig::default(),
                    tensors: HashMap::new(),
                    data_offset: 0,
                });
            }
        };

        let mmap = match unsafe { memmap2::Mmap::map(&file) } {
            Ok(m) => m,
            Err(_) => {
                return Ok(Self {
                    mmap: None,
                    config: ModelConfig::default(),
                    tensors: HashMap::new(),
                    data_offset: 0,
                });
            }
        };

        if mmap.len() < 16 {
            return Ok(Self {
                mmap: Some(mmap),
                config: ModelConfig::default(),
                tensors: HashMap::new(),
                data_offset: 0,
            });
        }

        let mut cursor = Cursor::new(&mmap[..]);
        let mut magic_buf = [0u8; 4];
        if cursor.read_exact(&mut magic_buf).is_err() || &magic_buf != b"GGUF" {
            return Ok(Self {
                mmap: None,
                config: ModelConfig::default(),
                tensors: HashMap::new(),
                data_offset: 0,
            });
        }

        let _version = cursor.read_u32::<LittleEndian>().unwrap_or(3);
        let tensor_count = cursor.read_u64::<LittleEndian>().unwrap_or(0);
        let metadata_kv_count = cursor.read_u64::<LittleEndian>().unwrap_or(0);

        if tensor_count > 100_000 || metadata_kv_count > 10_000 {
            return Ok(Self {
                mmap: Some(mmap),
                config: ModelConfig::default(),
                tensors: HashMap::new(),
                data_offset: 0,
            });
        }

        let mut config = ModelConfig::default();

        for _ in 0..metadata_kv_count {
            if let Ok(key) = read_gguf_string(&mut cursor) {
                if let Ok(value_type) = cursor.read_u32::<LittleEndian>() {
                    if let Ok(val) = read_gguf_metadata_value(&mut cursor, value_type) {
                        match key.as_str() {
                            "general.architecture" => {
                                if let MetadataValue::String(s) = &val {
                                    config.architecture = s.clone();
                                }
                            }
                            "pseudo.context_length"
                            | "llama.context_length"
                            | "general.context_length"
                            | "qwen2.context_length" => {
                                if let MetadataValue::Uint(v) = val {
                                    config.context_length = v as usize;
                                } else if let MetadataValue::Int(v) = val {
                                    config.context_length = v as usize;
                                }
                            }
                            "pseudo.embedding_length"
                            | "llama.embedding_length"
                            | "general.embedding_length"
                            | "qwen2.embedding_length" => {
                                if let MetadataValue::Uint(v) = val {
                                    config.embedding_length = v as usize;
                                } else if let MetadataValue::Int(v) = val {
                                    config.embedding_length = v as usize;
                                }
                            }
                            "pseudo.block_count"
                            | "llama.block_count"
                            | "general.block_count"
                            | "qwen2.block_count" => {
                                if let MetadataValue::Uint(v) = val {
                                    config.block_count = v as usize;
                                } else if let MetadataValue::Int(v) = val {
                                    config.block_count = v as usize;
                                }
                            }
                            "pseudo.feed_forward_length"
                            | "llama.feed_forward_length"
                            | "general.feed_forward_length"
                            | "qwen2.feed_forward_length" => {
                                if let MetadataValue::Uint(v) = val {
                                    config.feed_forward_length = v as usize;
                                } else if let MetadataValue::Int(v) = val {
                                    config.feed_forward_length = v as usize;
                                }
                            }
                            "pseudo.attention.head_count"
                            | "llama.attention.head_count"
                            | "general.head_count"
                            | "qwen2.attention.head_count" => {
                                if let MetadataValue::Uint(v) = val {
                                    config.head_count = v as usize;
                                } else if let MetadataValue::Int(v) = val {
                                    config.head_count = v as usize;
                                }
                            }
                            "pseudo.attention.head_count_kv"
                            | "llama.attention.head_count_kv"
                            | "general.head_count_kv"
                            | "qwen2.attention.head_count_kv" => {
                                if let MetadataValue::Uint(v) = val {
                                    config.head_count_kv = v as usize;
                                } else if let MetadataValue::Int(v) = val {
                                    config.head_count_kv = v as usize;
                                }
                            }
                            "pseudo.attention.layer_norm_rms_epsilon"
                            | "llama.attention.layer_norm_rms_epsilon"
                            | "qwen2.attention.layer_norm_rms_epsilon"
                            | "general.rms_norm_eps"
                            | "pseudo.rms_norm_eps" => {
                                if let MetadataValue::Float32(f) = val {
                                    config.rms_norm_eps = f;
                                }
                            }
                            "pseudo.rope.freq_base"
                            | "llama.rope.freq_base"
                            | "qwen2.rope.freq_base"
                            | "general.rope_freq_base"
                            | "pseudo.freq_base" => {
                                if let MetadataValue::Float32(f) = val {
                                    config.rope_freq_base = f;
                                }
                            }
                            "pseudo.vocab_size" | "llama.vocab_size" | "general.vocab_size"
                            | "qwen2.vocab_size" => {
                                if let MetadataValue::Uint(v) = val {
                                    config.vocab_size = v as usize;
                                } else if let MetadataValue::Int(v) = val {
                                    config.vocab_size = v as usize;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let mut tensors = HashMap::new();
        for _ in 0..tensor_count {
            if let Ok(name) = read_gguf_string(&mut cursor) {
                if let Ok(n_dimensions) = cursor.read_u32::<LittleEndian>() {
                    if n_dimensions > 8 {
                        continue;
                    }
                    let mut dimensions = Vec::with_capacity(n_dimensions as usize);
                    let mut valid_dims = true;
                    for _ in 0..n_dimensions {
                        if let Ok(dim) = cursor.read_u64::<LittleEndian>() {
                            if dim > 100_000 {
                                valid_dims = false;
                            }
                            dimensions.push(dim as usize);
                        } else {
                            valid_dims = false;
                        }
                    }
                    if !valid_dims {
                        continue;
                    }
                    if let Ok(type_val) = cursor.read_u32::<LittleEndian>() {
                        let tensor_type =
                            GgufTensorType::from_u32(type_val).unwrap_or(GgufTensorType::F32);
                        if let Ok(offset) = cursor.read_u64::<LittleEndian>() {
                            let num_elements: usize = dimensions.iter().product();
                            if num_elements > 100_000_000 {
                                continue;
                            }
                            let size_bytes = match tensor_type {
                                GgufTensorType::F32 => num_elements * 4,
                                GgufTensorType::F16 | GgufTensorType::Q4_0 => num_elements * 2,
                                GgufTensorType::Q8_0 => num_elements,
                                GgufTensorType::TQ2_0 => {
                                    let in_dim = dimensions.first().copied().unwrap_or(1);
                                    let out_dim: usize =
                                        dimensions.iter().skip(1).product::<usize>().max(1);
                                    let packed_bytes = out_dim * in_dim.div_ceil(4);
                                    let scales_offset = (packed_bytes + 3) & !3;
                                    scales_offset + out_dim * 4
                                }
                            };
                            tensors.insert(
                                name.clone(),
                                GgufTensorInfo {
                                    name,
                                    dimensions,
                                    tensor_type,
                                    offset,
                                    size_bytes,
                                },
                            );
                        }
                    }
                }
            }
        }

        let current_pos = cursor.position() as usize;
        let alignment = 32;
        let data_offset = (current_pos + alignment - 1) & !(alignment - 1);

        if let Some(out_info) = tensors.get("output.weight") {
            if out_info.dimensions.len() >= 2 {
                config.vocab_size = if out_info.dimensions[1] != config.embedding_length {
                    out_info.dimensions[1]
                } else {
                    out_info.dimensions[0]
                };
            }
        }

        Ok(Self {
            mmap: Some(mmap),
            config,
            tensors,
            data_offset,
        })
    }

    pub fn get_tensor_info(&self, name: &str) -> Option<&GgufTensorInfo> {
        self.tensors.get(name)
    }

    pub fn get_tensor_f32(&self, name: &str) -> Option<Vec<f32>> {
        let bytes = self.get_tensor_bytes(name)?;
        if bytes.len() % 4 != 0 {
            return None;
        }
        let mut floats = vec![0.0f32; bytes.len() / 4];
        for (i, chunk) in bytes.chunks_exact(4).enumerate() {
            floats[i] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        Some(floats)
    }

    pub fn get_tensor_f32_slice(&self, name: &str) -> Option<&[f32]> {
        let bytes = self.get_tensor_bytes(name)?;
        if !bytes.len().is_multiple_of(4)
            || !(bytes.as_ptr() as usize).is_multiple_of(std::mem::align_of::<f32>())
        {
            return None;
        }
        let len = bytes.len() / 4;
        Some(unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const f32, len) })
    }

    pub fn get_tensor_tq2_0(
        &self,
        name: &str,
        in_dim: usize,
        out_dim: usize,
    ) -> Option<(&[u8], Vec<f32>)> {
        let bytes = self.get_tensor_bytes(name)?;
        let packed_bytes = out_dim * in_dim.div_ceil(4);
        if bytes.len() < packed_bytes {
            return None;
        }
        let packed = &bytes[..packed_bytes];
        let scales_offset = (packed_bytes + 3) & !3;
        let mut scales = vec![1.0; out_dim];
        if bytes.len() >= scales_offset + out_dim * 4 {
            let scale_bytes = &bytes[scales_offset..scales_offset + out_dim * 4];
            for (i, chunk) in scale_bytes.chunks_exact(4).enumerate() {
                if i < out_dim {
                    scales[i] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                }
            }
        }
        Some((packed, scales))
    }

    pub fn get_tensor_bytes(&self, name: &str) -> Option<&[u8]> {
        let mmap = self.mmap.as_ref()?;
        let info = self.tensors.get(name)?;
        let start = self.data_offset + info.offset as usize;
        let end = start + info.size_bytes;
        if end <= mmap.len() {
            Some(&mmap[start..end])
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
enum MetadataValue {
    Uint(u64),
    Int(i64),
    Float32(f32),
    Float64(()),
    String(String),
    Array(()),
}

fn read_gguf_string(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
    let len = cursor
        .read_u64::<LittleEndian>()
        .map_err(|_| "Failed to read string length")?;
    if len > 1_048_576 {
        return Err(format!("GGUF string length too large: {}", len));
    }
    let mut buf = vec![0u8; len as usize];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| "Failed to read string bytes")?;
    String::from_utf8(buf).map_err(|e| format!("Invalid UTF-8 in GGUF string: {}", e))
}

fn read_gguf_metadata_value(
    cursor: &mut Cursor<&[u8]>,
    value_type: u32,
) -> Result<MetadataValue, String> {
    match value_type {
        0 => Ok(MetadataValue::Uint(
            cursor.read_u8().map_err(|_| "Metadata u8")? as u64,
        )),
        1 => Ok(MetadataValue::Int(
            cursor.read_i8().map_err(|_| "Metadata i8")? as i64,
        )),
        2 => Ok(MetadataValue::Uint(
            cursor
                .read_u16::<LittleEndian>()
                .map_err(|_| "Metadata u16")? as u64,
        )),
        3 => Ok(MetadataValue::Int(
            cursor
                .read_i16::<LittleEndian>()
                .map_err(|_| "Metadata u16")? as i64,
        )),
        4 => Ok(MetadataValue::Uint(
            cursor
                .read_u32::<LittleEndian>()
                .map_err(|_| "Metadata u32")? as u64,
        )),
        5 => Ok(MetadataValue::Int(
            cursor
                .read_i32::<LittleEndian>()
                .map_err(|_| "Metadata i32")? as i64,
        )),
        6 => {
            let val = cursor
                .read_f32::<LittleEndian>()
                .map_err(|_| "Metadata f32")?;
            Ok(MetadataValue::Float32(val))
        }
        7 => {
            let b = cursor.read_u8().map_err(|_| "Metadata bool")?;
            Ok(MetadataValue::Uint(if b != 0 { 1 } else { 0 }))
        }
        8 => Ok(MetadataValue::String(read_gguf_string(cursor)?)),
        9 => {
            let elem_type = cursor
                .read_u32::<LittleEndian>()
                .map_err(|_| "Array elem type")?;
            let len = cursor.read_u64::<LittleEndian>().map_err(|_| "Array len")?;
            if len > 100_000 {
                return Err(format!("Array length too large: {}", len));
            }
            let elem_size = match elem_type {
                0..=3 => 2,
                4..=7 => 4,
                8 => {
                    for _ in 0..len {
                        let _ = read_gguf_string(cursor);
                    }
                    return Ok(MetadataValue::Array(()));
                }
                10..=12 => 8,
                _ => 1,
            };
            let bytes_to_skip = len as usize * elem_size;
            let mut discard = vec![0u8; bytes_to_skip];
            cursor
                .read_exact(&mut discard)
                .map_err(|_| "Failed to skip array elements")?;
            Ok(MetadataValue::Array(()))
        }
        10 => Ok(MetadataValue::Uint(
            cursor
                .read_u64::<LittleEndian>()
                .map_err(|_| "Metadata u64")?,
        )),
        11 => Ok(MetadataValue::Int(
            cursor
                .read_i64::<LittleEndian>()
                .map_err(|_| "Metadata i64")?,
        )),
        12 => {
            let _ = cursor
                .read_f64::<LittleEndian>()
                .map_err(|_| "Metadata f64")?;
            Ok(MetadataValue::Float64(()))
        }
        _ => Err(format!("Unknown GGUF metadata value type: {}", value_type)),
    }
}
