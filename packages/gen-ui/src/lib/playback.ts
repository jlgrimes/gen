import Soundfont from 'soundfont-player';

export interface PlaybackNote {
  midiNote: number;
  startTime: number;  // in beats
  duration: number;   // in beats
}

export interface PlaybackData {
  tempo: number;      // BPM
  notes: PlaybackNote[];
}

export type PlaybackState = 'stopped' | 'playing' | 'paused';

// Helper to convert MIDI note number to note name (e.g., 60 -> "C4")
function midiToNoteName(midi: number): string {
  const noteNames = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
  const octave = Math.floor(midi / 12) - 1;
  const noteName = noteNames[midi % 12];
  return `${noteName}${octave}`;
}

export class PlaybackEngine {
  private audioContext: AudioContext;
  private instrument: any; // Soundfont.Player
  private instrumentName: string | null = null;
  private startTime: number = 0;
  private pausedAt: number = 0;
  private currentBeat: number = 0;
  private state: PlaybackState = 'stopped';
  private animationFrame?: number;
  private onProgressCallback?: (beat: number) => void;
  private onEndCallback?: () => void;
  private totalBeats: number = 0;
  private currentData: PlaybackData | null = null;

  constructor() {
    this.audioContext = new AudioContext();
  }

  async loadInstrument(instrumentName: string): Promise<void> {
    // Only load if different instrument
    if (this.instrumentName === instrumentName && this.instrument) {
      return;
    }

    this.instrumentName = instrumentName;
    try {
      this.instrument = await Soundfont.instrument(
        this.audioContext,
        instrumentName,
        { soundfont: 'MusyngKite' } // High-quality soundfont from CDN
      );
    } catch (err) {
      console.error('Failed to load instrument:', err);
      throw err;
    }
  }

  async play(
    data: PlaybackData,
    onProgress?: (beat: number) => void,
    onEnd?: () => void
  ): Promise<void> {
    // Recreate audio context if it was closed
    if (this.audioContext.state === 'closed') {
      this.audioContext = new AudioContext();
      this.instrument = null; // Need to reload instrument with new context
      this.instrumentName = null;
    }

    if (!this.instrument) {
      throw new Error('Instrument not loaded');
    }

    // Resume audio context if suspended (browser autoplay policy)
    if (this.audioContext.state === 'suspended') {
      await this.audioContext.resume();
    }

    // Verify audio context is running
    if (this.audioContext.state !== 'running') {
      throw new Error('Audio context failed to start');
    }

    this.state = 'playing';
    this.currentData = data;
    this.onProgressCallback = onProgress;
    this.onEndCallback = onEnd;
    this.startTime = this.audioContext.currentTime - this.pausedAt;
    const beatsPerSecond = data.tempo / 60;

    // Calculate total duration
    this.totalBeats = Math.max(
      ...data.notes.map(n => n.startTime + n.duration),
      0
    );

    // Schedule all notes
    // Add small buffer (50ms) to ensure first note plays
    const startBuffer = 0.05;
    for (const note of data.notes) {
      const timeInSeconds = note.startTime / beatsPerSecond;
      const durationInSeconds = note.duration / beatsPerSecond;
      const absoluteTime = this.audioContext.currentTime + startBuffer + timeInSeconds - this.pausedAt;

      // Only schedule notes that haven't passed yet
      if (absoluteTime > this.audioContext.currentTime) {
        const noteName = midiToNoteName(note.midiNote);
        this.instrument.play(
          noteName,
          absoluteTime,
          {
            duration: durationInSeconds,
            gain: 1.0
          }
        );
      }
    }

    // Progress tracking
    const updateProgress = () => {
      if (this.state !== 'playing') return;

      const elapsed = this.audioContext.currentTime - this.startTime;
      this.currentBeat = elapsed * beatsPerSecond;

      if (this.onProgressCallback) {
        this.onProgressCallback(this.currentBeat);
      }

      if (this.currentBeat < this.totalBeats) {
        this.animationFrame = requestAnimationFrame(updateProgress);
      } else {
        this.stop();
        if (this.onEndCallback) {
          this.onEndCallback();
        }
      }
    };
    updateProgress();
  }

  pause(): void {
    if (this.state !== 'playing') return;

    this.state = 'paused';
    this.pausedAt = this.audioContext.currentTime - this.startTime;

    if (this.animationFrame) {
      cancelAnimationFrame(this.animationFrame);
      this.animationFrame = undefined;
    }

    // Stop all scheduled notes
    this.instrument?.stop();
  }

  stop(): void {
    this.state = 'stopped';
    this.pausedAt = 0;
    this.currentBeat = 0;

    if (this.animationFrame) {
      cancelAnimationFrame(this.animationFrame);
      this.animationFrame = undefined;
    }

    // Stop all scheduled notes
    this.instrument?.stop();
  }

  async seek(beat: number): Promise<void> {
    const wasPlaying = this.state === 'playing';

    // Stop current playback
    this.stop();

    if (!this.currentData) return;

    // Set position
    this.pausedAt = beat / (this.currentData.tempo / 60);
    this.currentBeat = beat;

    // Resume if was playing
    if (wasPlaying) {
      await this.play(this.currentData, this.onProgressCallback, this.onEndCallback);
    }
  }

  getState(): PlaybackState {
    return this.state;
  }

  getCurrentBeat(): number {
    return this.currentBeat;
  }

  getTotalBeats(): number {
    return this.totalBeats;
  }

  dispose(): void {
    this.stop();
    if (this.audioContext.state !== 'closed') {
      this.audioContext.close();
    }
  }
}
