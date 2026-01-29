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
  transposeKey?: 'C' | 'Bb' | 'Eb' | 'F';  // Which key to transpose to (C = concert pitch)
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

export interface PlaybackNote {
  midiNote: number;         // Concert pitch (for playback)
  displayMidiNote: number;  // Display pitch (transposed, matches sheet music)
  startTime: number;        // in beats
  duration: number;         // in beats
}

export interface PlaybackChord {
  midiNotes: number[];  // multiple MIDI notes played simultaneously
  startTime: number;    // in beats
  duration: number;     // in beats
}

export interface PlaybackData {
  tempo: number;      // BPM
  notes: PlaybackNote[];
  chords: PlaybackChord[];  // chord accompaniment (always piano)
}

export interface PlaybackResult {
  status: 'success' | 'error';
  data?: PlaybackData;
  error?: CompileError;
}

export interface PlaybackAdapter {
  generatePlaybackData(source: string, options: CompileOptions): Promise<PlaybackResult>;
}
