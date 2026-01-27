export type InstrumentGroup = 'eb' | 'bb';

export interface ModPoint {
  line: number;
  octaveShift: number;  // +1 or -1
}

export interface ModPoints {
  eb: ModPoint[];
  bb: ModPoint[];
}

export interface CompileOptions {
  clef: 'treble' | 'bass';
  octaveShift: number;
  instrumentGroup?: InstrumentGroup;
}

export interface CompileResult {
  status: 'success' | 'error';
  xml?: string;
  error?: CompileError;
}

export interface CompileError {
  message: string;
  line: number | null;
  column: number | null;
}

export interface ScoreInfo {
  name: string;
  content: string;
}

export interface CompilerAdapter {
  compile(source: string, options: CompileOptions): Promise<CompileResult>;
}

export interface FileAdapter {
  savePdf(data: Uint8Array, suggestedName: string): Promise<void>;
}
