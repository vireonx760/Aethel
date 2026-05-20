use crate::gui::paint::{FIRST_CUSTOM_SHADER_MODE, ShaderMode};

#[derive(Debug, Clone, PartialEq)]
pub struct CustomShader {
    pub name: String,
    pub mode: ShaderMode,
    pub wgsl_source: String,
    pub vertex_entry: String,
    pub fragment_entry: String,
}

impl CustomShader {
    pub fn wgsl(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            mode: ShaderMode::Custom(FIRST_CUSTOM_SHADER_MODE),
            wgsl_source: source.into(),
            vertex_entry: "vs_main".to_string(),
            fragment_entry: "fs_main".to_string(),
        }
    }

    pub fn entries(mut self, vertex: impl Into<String>, fragment: impl Into<String>) -> Self {
        self.vertex_entry = vertex.into();
        self.fragment_entry = fragment.into();
        self
    }

    pub fn with_mode(mut self, mode: ShaderMode) -> Self {
        self.mode = mode;
        self
    }
}

#[derive(Debug, Clone)]
pub struct CustomShaderRegistry {
    next_mode: f32,
    shaders: Vec<CustomShader>,
}

impl CustomShaderRegistry {
    pub fn new() -> Self {
        Self {
            next_mode: FIRST_CUSTOM_SHADER_MODE,
            shaders: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next_mode: FIRST_CUSTOM_SHADER_MODE,
            shaders: Vec::with_capacity(capacity),
        }
    }

    pub fn register(&mut self, mut shader: CustomShader) -> ShaderMode {
        let mode = ShaderMode::Custom(self.next_mode);
        self.next_mode += 1.0;
        shader.mode = mode;
        self.shaders.push(shader);
        mode
    }

    pub fn register_wgsl(
        &mut self,
        name: impl Into<String>,
        source: impl Into<String>,
    ) -> ShaderMode {
        self.register(CustomShader::wgsl(name, source))
    }

    pub fn get(&self, mode: ShaderMode) -> Option<&CustomShader> {
        let ShaderMode::Custom(value) = mode else {
            return None;
        };
        self.shaders
            .iter()
            .find(|shader| shader.mode.as_f32() == value)
    }

    pub fn iter(&self) -> impl Iterator<Item = &CustomShader> {
        self.shaders.iter()
    }

    pub fn len(&self) -> usize {
        self.shaders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shaders.is_empty()
    }
}

impl Default for CustomShaderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_assigns_stable_custom_modes() {
        let mut registry = CustomShaderRegistry::new();
        let first = registry.register_wgsl("a", "@fragment fn fs_main() {}");
        let second = registry.register_wgsl("b", "@fragment fn fs_main() {}");

        assert_eq!(first.as_f32(), FIRST_CUSTOM_SHADER_MODE);
        assert_eq!(second.as_f32(), FIRST_CUSTOM_SHADER_MODE + 1.0);
        assert_eq!(registry.len(), 2);
        assert!(matches!(registry.get(first), Some(shader) if shader.name == "a"));
    }
}
