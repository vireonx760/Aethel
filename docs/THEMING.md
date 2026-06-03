# Theming

The public theme facade is available through the prelude:

```rust
use aethel_gui::prelude::*;
```

Current theme types:

- `Color`
- `Theme`
- `Style`
- `SurfaceStyle`
- `TextStyle`
- `VisualState`

`UiState` stores the active theme:

```rust
let mut state = UiState::new();
state.set_theme(Theme::dark());
```

The default `run_ui` path uses `Theme::dark()`. Widget-specific style override APIs will be expanded after the 0.3 developer-preview surface settles.
