import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import type { CompilerAdapter, FileAdapter, CompileResult } from 'gen-ui';

export const tauriCompiler: CompilerAdapter = {
  async compile(source, options) {
    return invoke<CompileResult>('compile_gen_with_mod_points', {
      source,
      clef: options.clef,
      octaveShift: options.octaveShift,
      instrumentGroup: options.instrumentGroup ?? null,
    });
  },
};

export const tauriFiles: FileAdapter = {
  async savePdf(data, suggestedName) {
    const filePath = await save({
      defaultPath: suggestedName,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    });
    if (filePath) {
      await writeFile(filePath, data);
    }
  },
};
