# @gen/docs

Documentation package for the Gen music notation language.

## Installation

```bash
npm install @gen/docs
# or
pnpm add @gen/docs
```

## Usage

```typescript
import { docs, getDocPath, listDocs } from '@gen/docs';

// Get a specific doc path
const basicsPath = getDocPath('basics');

// List all available docs
const allDocs = listDocs();

// Access doc paths directly
console.log(docs.v1.basics); // '/docs/v1/gen basics.md'
```

## Accessing Documentation Files

The markdown files are included in the package and can be accessed via the exported paths. If you're using a bundler like Vite or webpack, you can import them:

```typescript
import { docs } from '@gen/docs';

// In a browser environment with proper bundler config:
const response = await fetch(docs.v1.basics);
const markdown = await response.text();
```

## Available Documentation

- **Welcome** - Introduction to Gen
- **Gen Basics** - Language syntax and fundamentals
- **Examples** - Example scores and patterns
- **Compiler** - Compiler architecture and API
- **UI** - Gen UI component library

## Package Contents

```
@gen/docs/
├── dist/          # Compiled TypeScript
├── docs/          # Markdown documentation files
│   ├── v1/
│   │   ├── gen basics.md
│   │   ├── examples.md
│   │   ├── compiler.md
│   │   └── gen-ui.md
│   └── Welcome.md
└── src/           # TypeScript source
```
