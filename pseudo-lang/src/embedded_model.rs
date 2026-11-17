use std::fs;
use std::io::Write;
use std::path::PathBuf;

// Bundling the ~10MB 33M-TQ2_0 GGUF inside the binary means `psudo synth` works out of the box—
// I don't want users hunting for model weights just to test a local run.
pub const EMBEDDED_MODEL_BYTES: &[u8] = include_bytes!("../models/pseudo-33m-tq2_0.gguf");
pub const EMBEDDED_MODEL_NAME: &str = "pseudo-33m-tq2_0.gguf";

/// Ensures an on-disk copy of the embedded model is available for the engine daemon.
/// Writes `EMBEDDED_MODEL_BYTES` to `.pseudo/state/models/pseudo-33m-tq2_0.gguf` if missing or corrupted.
pub fn ensure_embedded_model_extracted() -> Result<PathBuf, String> {
    let target_dir = PathBuf::from(".pseudo/state/models");
    let target_path = target_dir.join(EMBEDDED_MODEL_NAME);

    if target_path.exists() {
        if let Ok(metadata) = fs::metadata(&target_path) {
            if metadata.len() == EMBEDDED_MODEL_BYTES.len() as u64 {
                return Ok(target_path);
            }
        }
    }

    if let Err(e) = fs::create_dir_all(&target_dir) {
        return Err(format!(
            "failed to create embedded model extraction directory '{}': {}",
            target_dir.display(),
            e
        ));
    }

    let temp_path = target_dir.join(format!("{}.tmp.{}", EMBEDDED_MODEL_NAME, std::process::id()));
    {
        let mut file = fs::File::create(&temp_path).map_err(|e| {
            format!(
                "failed to create temporary model file '{}': {}",
                temp_path.display(),
                e
            )
        })?;

        file.write_all(EMBEDDED_MODEL_BYTES).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            format!(
                "failed to write embedded model payload to '{}': {}",
                temp_path.display(),
                e
            )
        })?;

        file.flush().map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            format!(
                "failed to flush embedded model payload to '{}': {}",
                temp_path.display(),
                e
            )
        })?;
    }

    if let Err(e) = fs::rename(&temp_path, &target_path) {
        // Fallback for Windows cross-file/replace issues
        let _ = fs::remove_file(&target_path);
        if let Err(rename_err) = fs::rename(&temp_path, &target_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!(
                "failed to atomically place model into '{}': {} (initial: {})",
                target_path.display(),
                rename_err,
                e
            ));
        }
    }

    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_model_constants() {
        assert_eq!(EMBEDDED_MODEL_NAME, "pseudo-33m-tq2_0.gguf");
        assert_eq!(EMBEDDED_MODEL_BYTES.len(), 24353504);
        // Validate GGUF header magic: 'G' 'G' 'U' 'F' = 0x46554747
        assert_eq!(&EMBEDDED_MODEL_BYTES[0..4], b"GGUF");
    }

    #[test]
    fn test_embedded_model_extraction() {
        let extracted_path = ensure_embedded_model_extracted().expect("extraction should succeed");
        assert!(extracted_path.exists());
        let meta = fs::metadata(&extracted_path).expect("metadata should exist");
        assert_eq!(meta.len(), EMBEDDED_MODEL_BYTES.len() as u64);

        // Second call should return immediately without re-extracting
        let second_call = ensure_embedded_model_extracted().expect("re-check should succeed");
        assert_eq!(second_call, extracted_path);
    }
}
