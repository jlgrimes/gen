import init, { compile_with_options, list_scores } from 'gen-wasm';
import type { CompilerAdapter, FileAdapter, ScoreInfo } from 'gen-ui';

let initialized = false;

async function ensureInit() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

export const wasmCompiler: CompilerAdapter = {
  async compile(source, options) {
    await ensureInit();
    try {
      const xml = compile_with_options(source, options.clef, options.octaveShift);
      return { status: 'success', xml };
    } catch (e: unknown) {
      // WASM throws error message as JSON string
      const errorStr = e instanceof Error ? e.message : String(e);
      try {
        return { status: 'error', error: JSON.parse(errorStr) };
      } catch {
        return { status: 'error', error: { message: errorStr, line: null, column: null } };
      }
    }
  },
  async listScores() {
    await ensureInit();
    return list_scores() as ScoreInfo[];
  },
};

export const browserFiles: FileAdapter = {
  async savePdf(data, suggestedName) {
    const blob = new Blob([new Uint8Array(data)], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = suggestedName;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  },
};
