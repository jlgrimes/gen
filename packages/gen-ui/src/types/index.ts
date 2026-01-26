export interface CompileOptions {
  clef: 'treble' | 'bass';
  octaveShift: number;
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
  listScores(): Promise<ScoreInfo[]>;
}

export interface FileAdapter {
  savePdf(data: Uint8Array, suggestedName: string): Promise<void>;
}
