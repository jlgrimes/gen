import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tauriCompiler } from './adapters';
import type { CompileOptions, CompileResult } from 'gen-ui';

// Mock the Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('tauriCompiler', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should pass all parameters including transposeKey to Tauri invoke', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    const mockResult: CompileResult = {
      status: 'success',
      xml: '<musicxml>test</musicxml>',
    };
    vi.mocked(invoke).mockResolvedValue(mockResult);

    const source = '---\ntitle: Test\n---\nC D E F';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
      instrumentGroup: 'bb',
      transposeKey: 'Bb',
    };

    const result = await tauriCompiler.compile(source, options);

    expect(invoke).toHaveBeenCalledWith('compile_gen_with_mod_points', {
      source,
      clef: 'treble',
      octaveShift: 0,
      instrumentGroup: 'Bb',
      transposeKey: 'Bb',
    });
    expect(result).toEqual(mockResult);
  });

  it('should pass null for optional parameters when not provided', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    const mockResult: CompileResult = {
      status: 'success',
      xml: '<musicxml>test</musicxml>',
    };
    vi.mocked(invoke).mockResolvedValue(mockResult);

    const source = '---\ntitle: Test\n---\nC D E F';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
    };

    await tauriCompiler.compile(source, options);

    expect(invoke).toHaveBeenCalledWith('compile_gen_with_mod_points', {
      source,
      clef: 'treble',
      octaveShift: 0,
      instrumentGroup: null,
      transposeKey: null,
    });
  });

  it('should handle transposeKey without instrumentGroup', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    const mockResult: CompileResult = {
      status: 'success',
      xml: '<musicxml>test</musicxml>',
    };
    vi.mocked(invoke).mockResolvedValue(mockResult);

    const source = '---\ntitle: Test\n---\nC D E F';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
      transposeKey: 'Eb',
    };

    await tauriCompiler.compile(source, options);

    expect(invoke).toHaveBeenCalledWith('compile_gen_with_mod_points', {
      source,
      clef: 'treble',
      octaveShift: 0,
      instrumentGroup: null,
      transposeKey: 'Eb',
    });
  });

  it('should handle all transpose key options', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    const mockResult: CompileResult = {
      status: 'success',
      xml: '<musicxml>test</musicxml>',
    };
    vi.mocked(invoke).mockResolvedValue(mockResult);

    const source = '---\ntitle: Test\n---\nC D E F';
    const transposeKeys: Array<'C' | 'Bb' | 'Eb' | 'F'> = ['C', 'Bb', 'Eb', 'F'];

    for (const transposeKey of transposeKeys) {
      vi.clearAllMocks();

      const options: CompileOptions = {
        clef: 'treble',
        octaveShift: 0,
        transposeKey,
      };

      await tauriCompiler.compile(source, options);

      expect(invoke).toHaveBeenCalledWith('compile_gen_with_mod_points', {
        source,
        clef: 'treble',
        octaveShift: 0,
        instrumentGroup: null,
        transposeKey,
      });
    }
  });

  it('should handle compilation errors', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    const mockError: CompileResult = {
      status: 'error',
      error: {
        message: 'Invalid syntax',
        line: 5,
        column: 10,
      },
    };
    vi.mocked(invoke).mockResolvedValue(mockError);

    const source = '---\ntitle: Test\n---\nINVALID';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
      transposeKey: 'Bb',
    };

    const result = await tauriCompiler.compile(source, options);

    expect(result).toEqual(mockError);
  });

  it('should correctly pass octaveShift parameter', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    const mockResult: CompileResult = {
      status: 'success',
      xml: '<musicxml>test</musicxml>',
    };
    vi.mocked(invoke).mockResolvedValue(mockResult);

    const source = '---\ntitle: Test\n---\nC D E F';
    const octaveShifts = [-2, -1, 0, 1, 2];

    for (const octaveShift of octaveShifts) {
      vi.clearAllMocks();

      const options: CompileOptions = {
        clef: 'treble',
        octaveShift,
        transposeKey: 'C',
      };

      await tauriCompiler.compile(source, options);

      expect(invoke).toHaveBeenCalledWith('compile_gen_with_mod_points', {
        source,
        clef: 'treble',
        octaveShift,
        instrumentGroup: null,
        transposeKey: 'C',
      });
    }
  });

  it('should pass all parameters in correct order and format', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    const mockResult: CompileResult = {
      status: 'success',
      xml: '<musicxml>test</musicxml>',
    };
    vi.mocked(invoke).mockResolvedValue(mockResult);

    const source = '---\ntitle: Test\n---\nC D E F';
    const options: CompileOptions = {
      clef: 'bass',
      octaveShift: -1,
      instrumentGroup: 'eb',
      transposeKey: 'F',
    };

    await tauriCompiler.compile(source, options);

    // Verify the exact call signature with all parameters
    expect(invoke).toHaveBeenCalledWith('compile_gen_with_mod_points', {
      source: source,
      clef: 'bass',
      octaveShift: -1,
      instrumentGroup: 'Eb',
      transposeKey: 'F',  // Ensure transposeKey is passed correctly
    });
  });

  it('should handle instrumentGroup without transposeKey', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    const mockResult: CompileResult = {
      status: 'success',
      xml: '<musicxml>test</musicxml>',
    };
    vi.mocked(invoke).mockResolvedValue(mockResult);

    const source = '---\ntitle: Test\n---\nC D E F';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
      instrumentGroup: 'eb',
    };

    await tauriCompiler.compile(source, options);

    expect(invoke).toHaveBeenCalledWith('compile_gen_with_mod_points', {
      source,
      clef: 'treble',
      octaveShift: 0,
      instrumentGroup: 'Eb',
      transposeKey: null,
    });
  });

  it('should handle all clef types', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    const mockResult: CompileResult = {
      status: 'success',
      xml: '<musicxml>test</musicxml>',
    };
    vi.mocked(invoke).mockResolvedValue(mockResult);

    const source = '---\ntitle: Test\n---\nC D E F';
    const clefs: Array<'treble' | 'bass'> = ['treble', 'bass'];

    for (const clef of clefs) {
      vi.clearAllMocks();

      const options: CompileOptions = {
        clef,
        octaveShift: 0,
      };

      await tauriCompiler.compile(source, options);

      expect(invoke).toHaveBeenCalledWith('compile_gen_with_mod_points', {
        source,
        clef,
        octaveShift: 0,
        instrumentGroup: null,
        transposeKey: null,
      });
    }
  });
});
