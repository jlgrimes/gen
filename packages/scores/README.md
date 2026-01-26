# Gen Scores

This folder contains `.gen` score files that can be compiled to MusicXML.

## Structure

Organize your scores however you like:
```
scores/
├── my-compositions/
├── arrangements/
├── practice/
└── ...
```

## Compiling

From the repo root:
```bash
cd packages/gen-compiler
cargo run -- ../scores/your-file.gen output.xml
```
