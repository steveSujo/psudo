// Keeping Tensor as a plain shape + flat Vec<f32> avoids pulling heavy tensor-framework
// dependencies into a minimal, pure-Rust runtime.
pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

impl Tensor {
    pub fn new(shape: Vec<usize>, data: Vec<f32>) -> Self {
        Self { shape, data }
    }

    pub fn zeros(shape: Vec<usize>) -> Self {
        let size: usize = shape.iter().product();
        Self {
            shape,
            data: vec![0.0; size],
        }
    }

    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }
}
