# Public API Boundaries

The recommended 0.3.0 entry point is:

```rust
use aethel_gui::prelude::*;
```

## Stable Preview API

- `AethelGui`
- `Result`
- `Ui`, `UiState`, `Response`
- `Color`, `Theme`, `Style`, `Layout`
- Basic widgets exported from `widgets`

## Experimental API

Experimental exports live under `aethel_gui::experimental`. These APIs are useful but may change more freely:

- custom shader helpers
- GPU/frame statistics

## Low-Level Compatibility API

The legacy `core`, `gui`, and `gpu_core` modules remain public for compatibility with existing examples and tests. They should be treated as low-level runtime APIs until the 0.3 developer-preview surface is fully settled.
