# Gen UI - Desktop Application

Gen UI is a Tauri v2 + React desktop application for editing and viewing Gen scores as rendered sheet music.

## Features

- **Sidebar**: Browse embedded scores from the gen-scores library
- **Editor**: Real-time Gen syntax editing with monospace font
- **Sheet Music View**: Live rendering using OpenSheetMusicDisplay (OSMD)
- **Real-time Compilation**: 150ms debounced compilation as you type

## Architecture

```
gen-ui/
├── src/                      # React frontend
│   ├── App.tsx               # Main application component
│   ├── components/
│   │   └── ui/
│   │       └── sidebar.tsx   # Score browser sidebar
│   ├── lib/
│   │   └── utils.ts          # Tailwind utility functions
│   ├── index.css             # Global styles + Tailwind theme
│   └── main.tsx              # React entry point
├── src-tauri/                # Rust backend (Tauri)
│   ├── src/
│   │   ├── lib.rs            # Tauri commands
│   │   └── main.rs           # Entry point
│   ├── Cargo.toml            # Rust dependencies
│   └── tauri.conf.json       # Tauri configuration
├── package.json              # Node dependencies
└── vite.config.ts            # Vite build configuration
```

## Tech Stack

- **Tauri v2**: Desktop application framework (Rust backend)
- **React 18**: UI framework
- **TypeScript**: Type-safe JavaScript
- **Tailwind CSS v4**: Utility-first CSS
- **Vite**: Build tool and dev server
- **OpenSheetMusicDisplay**: MusicXML rendering library

## Tauri Commands

The Rust backend exposes these commands to the frontend:

```typescript
// Compile Gen source with validation
invoke<string>("compile_gen", { source: string })

// Compile without validation (for real-time editing)
invoke<string>("compile_gen_unchecked", { source: string })

// List all embedded scores
invoke<ScoreInfo[]>("list_scores")

// Get a specific score by name
invoke<ScoreInfo | null>("get_score", { name: string })
```

## Development

### Prerequisites
- Node.js 18+
- pnpm
- Rust toolchain
- Tauri CLI: `cargo install tauri-cli`

### Running Locally
```bash
cd packages/gen-ui
pnpm install
pnpm tauri dev
```

### Building for Production
```bash
pnpm tauri build
```

The built application will be in `src-tauri/target/release/`.

## Component Structure

### App.tsx
Main application with three panels:
1. **Sidebar** - Score browser (left)
2. **Editor** - Text editor for Gen source (middle)
3. **Sheet Music** - OSMD rendering (right)

Key behaviors:
- Loads embedded scores on mount
- Auto-selects first score
- Debounces compilation (150ms) for performance
- Displays compilation errors below editor

### Sidebar.tsx
Displays list of available scores with:
- Music icon header
- Clickable score list with file icons
- Visual selection state
- Empty state message

## Styling

Uses Tailwind CSS v4 with custom theme variables defined in `index.css`:

```css
@theme {
  --color-sidebar: #fafafa;
  --color-sidebar-foreground: #0a0a0a;
  --color-sidebar-border: #e5e5e5;
  --color-sidebar-accent: #f5f5f5;
  /* ... */
}
```

## Configuration

### Vite (vite.config.ts)
- React plugin with Fast Refresh
- Tailwind CSS plugin
- Path alias: `@/` → `./src/`
- Dev server on port 1420

### Tauri (tauri.conf.json)
- Application identifier
- Window configuration
- Build settings for different platforms

## Troubleshooting

### Tailwind not compiling
Ensure you're using Tailwind v4 syntax:
```css
@import "tailwindcss";
```
Not the v3 directives (`@tailwind base`, etc.)

### Compilation errors not showing
Check the browser console for detailed error messages from the Tauri backend.

### Sheet music not rendering
Ensure:
1. The Gen source is valid
2. No compilation errors (check error panel)
3. OSMD has finished loading (check network tab)
