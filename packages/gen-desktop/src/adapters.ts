import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import type {
  CompilerAdapter,
  FileAdapter,
  CompileResult,
  CompileOptions,
  PlaybackAdapter,
  PlaybackResult
} from 'gen-ui';

export const tauriCompiler: CompilerAdapter = {
  async compile(source: string, options: CompileOptions): Promise<CompileResult> {
    return invoke<CompileResult>('compile_gen_with_mod_points', {
      source,
      clef: options.clef,
      octaveShift: options.octaveShift,
      instrumentGroup: options.instrumentGroup ?? null,
      transposeKey: options.transposeKey ?? null,
    });
  },
};

export const tauriFiles: FileAdapter = {
  async savePdf(data: Uint8Array, suggestedName: string): Promise<void> {
    const filePath = await save({
      defaultPath: suggestedName,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    });
    if (filePath) {
      await writeFile(filePath, data);
    }
  },
};

export const tauriPlayback: PlaybackAdapter = {
  async generatePlaybackData(source: string, options: CompileOptions): Promise<PlaybackResult> {
    return invoke<PlaybackResult>('generate_playback_data', {
      source,
      clef: options.clef,
      octaveShift: options.octaveShift,
      instrumentGroup: options.instrumentGroup ?? null,
      transposeKey: options.transposeKey ?? null,
    });
  },
};
