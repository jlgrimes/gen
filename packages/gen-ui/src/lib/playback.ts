import Soundfont from 'soundfont-player';
import type { PlaybackData } from '../types';

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
  private instrument: any; // Soundfont.Player for melody
  private pianoInstrument: any; // Soundfont.Player for chords (always piano)
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
      // Load melody instrument
      this.instrument = await Soundfont.instrument(
        this.audioContext,
        instrumentName as any,
        { soundfont: 'MusyngKite' } // High-quality soundfont from CDN
      );

      // Always load piano for chords (if not already the melody instrument)
      if (instrumentName !== 'acoustic_grand_piano' && !this.pianoInstrument) {
        this.pianoInstrument = await Soundfont.instrument(
          this.audioContext,
          'acoustic_grand_piano' as any,
          { soundfont: 'MusyngKite' }
        );
      } else if (instrumentName === 'acoustic_grand_piano') {
        // If melody is piano, use same instrument for chords
        this.pianoInstrument = this.instrument;
      }
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
      this.pianoInstrument = null;
      this.instrumentName = null;
    }

    if (!this.instrument) {
      throw new Error('Instrument not loaded');
    }

    if (!this.pianoInstrument) {
      throw new Error('Piano instrument not loaded');
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
    console.log('[Playback] Tempo from backend:', data.tempo, 'BPM');
    const beatsPerSecond = data.tempo / 60;
    console.log('[Playback] Beats per second:', beatsPerSecond);

    // Calculate total duration
    const maxNoteEnd = data.notes.length > 0
      ? Math.max(...data.notes.map(n => n.startTime + n.duration))
      : 0;
    const maxChordEnd = data.chords.length > 0
      ? Math.max(...data.chords.map(c => c.startTime + c.duration))
      : 0;
    this.totalBeats = Math.max(maxNoteEnd, maxChordEnd, 0);

    // Schedule all melody notes
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

    // Schedule all chord notes (always on piano)
    for (const chord of data.chords) {
      const timeInSeconds = chord.startTime / beatsPerSecond;
      const durationInSeconds = chord.duration / beatsPerSecond;
      const absoluteTime = this.audioContext.currentTime + startBuffer + timeInSeconds - this.pausedAt;

      // Only schedule chords that haven't passed yet
      if (absoluteTime > this.audioContext.currentTime) {
        // Play each note in the chord
        for (const midiNote of chord.midiNotes) {
          const noteName = midiToNoteName(midiNote);
          this.pianoInstrument.play(
            noteName,
            absoluteTime,
            {
              duration: durationInSeconds,
              gain: 0.6  // Slightly softer for accompaniment
            }
          );
        }
      }
    }

    // Progress tracking
    const updateProgress = () => {
      if (this.state !== 'playing') return;

      const elapsed = this.audioContext.currentTime - this.startTime;
      // Subtract the start buffer to sync with audio scheduling
      const adjustedElapsed = Math.max(0, elapsed - startBuffer);
      this.currentBeat = adjustedElapsed * beatsPerSecond;

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
    if (this.pianoInstrument && this.pianoInstrument !== this.instrument) {
      this.pianoInstrument.stop();
    }
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
    if (this.pianoInstrument && this.pianoInstrument !== this.instrument) {
      this.pianoInstrument.stop();
    }
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
