import { describe, it, expect, vi, beforeEach } from 'vitest';
import { wasmCompiler } from './adapters';
import type { CompileOptions } from 'gen-ui';

// Mock the gen-wasm module
vi.mock('gen-wasm', () => ({
  default: vi.fn(() => Promise.resolve()),
  compile_with_mod_points: vi.fn(),
}));

describe('wasmCompiler', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should pass all parameters including transposeKey to compile_with_mod_points', async () => {
    const { compile_with_mod_points } = await import('gen-wasm');

    // Mock successful compilation
    vi.mocked(compile_with_mod_points).mockReturnValue('<musicxml>test</musicxml>');

    const source = '---\ntitle: Test\n---\nC D E F';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
      instrumentGroup: 'Bb',
      transposeKey: 'Bb',
    };

    const result = await wasmCompiler.compile(source, options);

    expect(compile_with_mod_points).toHaveBeenCalledWith(
      source,
      'treble',
      0,
      'Bb',
      'Bb'
    );
    expect(result).toEqual({
      status: 'success',
      xml: '<musicxml>test</musicxml>',
    });
  });

  it('should pass undefined for optional parameters when not provided', async () => {
    const { compile_with_mod_points } = await import('gen-wasm');

    vi.mocked(compile_with_mod_points).mockReturnValue('<musicxml>test</musicxml>');

    const source = '---\ntitle: Test\n---\nC D E F';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
    };

    await wasmCompiler.compile(source, options);

    expect(compile_with_mod_points).toHaveBeenCalledWith(
      source,
      'treble',
      0,
      undefined,
      undefined
    );
  });

  it('should handle transposeKey without instrumentGroup', async () => {
    const { compile_with_mod_points } = await import('gen-wasm');

    vi.mocked(compile_with_mod_points).mockReturnValue('<musicxml>test</musicxml>');

    const source = '---\ntitle: Test\n---\nC D E F';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
      transposeKey: 'Eb',
    };

    await wasmCompiler.compile(source, options);

    expect(compile_with_mod_points).toHaveBeenCalledWith(
      source,
      'treble',
      0,
      undefined,
      'Eb'
    );
  });

  it('should handle all transpose key options', async () => {
    const { compile_with_mod_points } = await import('gen-wasm');

    vi.mocked(compile_with_mod_points).mockReturnValue('<musicxml>test</musicxml>');

    const source = '---\ntitle: Test\n---\nC D E F';
    const transposeKeys: Array<'C' | 'Bb' | 'Eb' | 'F'> = ['C', 'Bb', 'Eb', 'F'];

    for (const transposeKey of transposeKeys) {
      vi.clearAllMocks();

      const options: CompileOptions = {
        clef: 'treble',
        octaveShift: 0,
        transposeKey,
      };

      await wasmCompiler.compile(source, options);

      expect(compile_with_mod_points).toHaveBeenCalledWith(
        source,
        'treble',
        0,
        undefined,
        transposeKey
      );
    }
  });

  it('should handle compilation errors', async () => {
    const { compile_with_mod_points } = await import('gen-wasm');

    const errorJson = JSON.stringify({
      message: 'Invalid syntax',
      line: 5,
      column: 10,
    });
    vi.mocked(compile_with_mod_points).mockImplementation(() => {
      throw new Error(errorJson);
    });

    const source = '---\ntitle: Test\n---\nINVALID';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
      transposeKey: 'Bb',
    };

    const result = await wasmCompiler.compile(source, options);

    expect(result).toEqual({
      status: 'error',
      error: {
        message: 'Invalid syntax',
        line: 5,
        column: 10,
      },
    });
  });

  it('should handle non-JSON error messages', async () => {
    const { compile_with_mod_points } = await import('gen-wasm');

    vi.mocked(compile_with_mod_points).mockImplementation(() => {
      throw new Error('Plain error message');
    });

    const source = '---\ntitle: Test\n---\nINVALID';
    const options: CompileOptions = {
      clef: 'treble',
      octaveShift: 0,
    };

    const result = await wasmCompiler.compile(source, options);

    expect(result).toEqual({
      status: 'error',
      error: {
        message: 'Plain error message',
        line: null,
        column: null,
      },
    });
  });

  it('should correctly pass octaveShift parameter', async () => {
    const { compile_with_mod_points } = await import('gen-wasm');

    vi.mocked(compile_with_mod_points).mockReturnValue('<musicxml>test</musicxml>');

    const source = '---\ntitle: Test\n---\nC D E F';
    const octaveShifts = [-2, -1, 0, 1, 2];

    for (const octaveShift of octaveShifts) {
      vi.clearAllMocks();

      const options: CompileOptions = {
        clef: 'treble',
        octaveShift,
        transposeKey: 'C',
      };

      await wasmCompiler.compile(source, options);

      expect(compile_with_mod_points).toHaveBeenCalledWith(
        source,
        'treble',
        octaveShift,
        undefined,
        'C'
      );
    }
  });

  it('should pass all parameters in correct order', async () => {
    const { compile_with_mod_points } = await import('gen-wasm');

    vi.mocked(compile_with_mod_points).mockReturnValue('<musicxml>test</musicxml>');

    const source = '---\ntitle: Test\n---\nC D E F';
    const options: CompileOptions = {
      clef: 'bass',
      octaveShift: -1,
      instrumentGroup: 'Eb',
      transposeKey: 'F',
    };

    await wasmCompiler.compile(source, options);

    // Verify the exact call signature
    expect(compile_with_mod_points).toHaveBeenCalledWith(
      source,      // 1st: source
      'bass',      // 2nd: clef
      -1,          // 3rd: octaveShift
      'Eb',        // 4th: instrumentGroup
      'F'          // 5th: transposeKey (THIS WAS MISSING BEFORE THE FIX)
    );
  });
});
