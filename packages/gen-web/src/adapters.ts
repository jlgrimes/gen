import init, { compile_with_mod_points, generate_playback_data } from 'gen-wasm';
import type { CompilerAdapter, FileAdapter, PlaybackAdapter } from 'gen-ui';

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
      const xml = compile_with_mod_points(
        source,
        options.clef,
        options.octaveShift,
        options.instrumentGroup ?? undefined,
        options.transposeKey ?? undefined
      );
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

export const wasmPlayback: PlaybackAdapter = {
  async generatePlaybackData(source, options) {
    await ensureInit();
    try {
      const json = generate_playback_data(
        source,
        options.clef,
        options.octaveShift,
        options.instrumentGroup ?? undefined,
        options.transposeKey ?? undefined
      );
      return { status: 'success', data: JSON.parse(json) };
    } catch (e: unknown) {
      const errorStr = e instanceof Error ? e.message : String(e);
      try {
        return { status: 'error', error: JSON.parse(errorStr) };
      } catch {
        return { status: 'error', error: { message: errorStr, line: null, column: null } };
      }
    }
  },
};
