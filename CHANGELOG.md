# Changelog

## 0.3.0 - Developer Preview

### Added

- Added `aethel_gui::prelude`.
- Added `AethelGui::run_ui` for immediate-style UI construction over retained widgets.
- Added `Ui`, `UiState`, `Response`, and stable key-based widget identity.
- Added public facade modules for color, style, layout, commands, and experimental APIs.
- Added `Separator` widget.
- Added `examples/basic_widgets.rs`.

### Changed

- Preserved low-level modules for compatibility while adding clearer public entry points.
- Immediate-style rebuilds reuse retained widget instances by stable `WidgetId`.

### Fixed

- Forwarded repaint intervals through `Flex` containers so focused nested text inputs keep blinking while idle.
