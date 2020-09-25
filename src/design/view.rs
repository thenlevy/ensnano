use ultraviolet::Mat4;

/// An object that stores the instances to be drawn to represent the desgin.
pub struct View {
    /// The model matrix of the design
    pub model_matrix: Mat4,
    /// True if there are new instances to be fetched
    was_updated: bool,
}

impl View {
    pub fn new() -> Self {
        Self {
            model_matrix: Mat4::identity(),
            was_updated: true,
        }
    }

    /// Return true if the view was updated since the last time this function was called
    pub fn was_updated(&mut self) -> bool {
        let ret = self.was_updated;
        self.was_updated = false;
        ret
    }

    /// Update the model matrix
    pub fn set_matrix(&mut self, matrix: Mat4) {
        self.model_matrix = matrix;
        self.was_updated = true;
    }
}

impl View {
    /// Return the model matrix
    pub fn get_model_matrix(&self) -> Mat4 {
        self.model_matrix
    }

}

