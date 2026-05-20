use glyphon::{Attrs, Buffer, Color, CustomGlyph, FontSystem, Shaping, TextArea, TextBounds};

const NO_CUSTOM_GLYPHS: &[CustomGlyph] = &[];

/// Sets finite text layout bounds using the `cosmic-text` 0.18 API.
#[inline]
pub fn set_buffer_size(buffer: &mut Buffer, font_system: &mut FontSystem, width: f32, height: f32) {
    buffer.set_size(font_system, Some(width), Some(height));
}

/// Updates buffer text with the default left alignment used by AethelGUI widgets.
#[inline]
pub fn set_buffer_text(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    text: &str,
    attrs: Attrs<'_>,
) {
    buffer.set_text(font_system, text, &attrs, Shaping::Advanced, None);
}

/// Shapes a text buffer without pruning retained layout data needed by warm frames.
#[inline]
pub fn shape_text(buffer: &mut Buffer, font_system: &mut FontSystem) {
    buffer.shape_until_scroll(font_system, false);
}

#[inline]
pub fn text_area<'a>(
    buffer: &'a Buffer,
    left: f32,
    top: f32,
    bounds: TextBounds,
    default_color: Color,
) -> TextArea<'a> {
    TextArea {
        buffer,
        left,
        top,
        scale: 1.0,
        bounds,
        default_color,
        custom_glyphs: NO_CUSTOM_GLYPHS,
    }
}
