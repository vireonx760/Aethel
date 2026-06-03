# Getting Started

AethelGUI 0.3.0 exposes a small developer-preview API through the prelude:

```rust
use aethel_gui::prelude::*;

fn main() -> Result {
    AethelGui::new().run_ui(|ui| {
        ui.label("Hello from AethelGUI");

        if ui.button("Click me").clicked() {
            println!("clicked");
        }
    })
}
```

The `run_ui` path is immediate-style on the outside and retained-first inside. Widgets are keyed by their label/key plus the current `ui.with_id(...)` scope. Matching keys reuse the previous widget instance, so focused text inputs and slider drag state survive rebuilds.

For lower-level integrations, the existing `GuiManager` API remains available through `AethelGui::run`.
