# Gen Scores

A Rust library that embeds `.gen` score files at compile time, making them available to the Gen UI application.

## How It Works

This package uses a `build.rs` script that:
1. Scans the `examples/` directory for `.gen` files
2. Embeds their contents into the compiled binary
3. Generates a static `SCORES` array accessible at runtime

## Adding New Scores

1. Create a `.gen` file in the `examples/` directory:
   ```
   packages/scores/examples/my-song.gen
   ```

2. Rebuild the project:
   ```bash
   # From repo root
   cargo build

   # Or from gen-ui for the desktop app
   cd packages/gen-ui
   pnpm tauri build
   ```

3. The score will now appear in the UI sidebar

## Directory Structure

```
packages/scores/
├── Cargo.toml       # Package manifest
├── build.rs         # Compile-time embedding script
├── src/
│   └── lib.rs       # Public API
└── examples/        # Score files go here
    ├── twinkle.gen
    └── ...
```

## API

```rust
use gen_scores::{Score, get_all_scores, get_score, list_scores};

// Get all embedded scores
let scores: Vec<Score> = get_all_scores();

// Get a specific score by name
let twinkle: Option<Score> = get_score("twinkle.gen");

// List all score names
let names: Vec<&str> = list_scores();
```

## Score Struct

```rust
pub struct Score {
    pub name: String,    // Filename (e.g., "twinkle.gen")
    pub content: String, // Full Gen source code
}
```

## Note

Scores are embedded at **compile time**. Any changes to score files require rebuilding the project to take effect.
