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
      console.log('Instrument already loaded:', instrumentName);
      return;
    }

    console.log('Loading instrument:', instrumentName);
    console.log('Audio context before load:', this.audioContext.state);
    this.instrumentName = instrumentName;
    try {
      this.instrument = await Soundfont.instrument(
        this.audioContext,
        instrumentName,
        { soundfont: 'MusyngKite' } // High-quality soundfont from CDN
      );
      console.log('Instrument loaded successfully:', instrumentName);
      console.log('Instrument object:', this.instrument);
      console.log('Instrument methods:', Object.keys(this.instrument));
      console.log('Audio context after load:', this.audioContext.state);
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
    console.log('Play called with data:', data);
    console.log('Instrument:', this.instrument);
    console.log('Audio context state:', this.audioContext.state);
    console.log('Audio context currentTime:', this.audioContext.currentTime);

    // Recreate audio context if it was closed
    if (this.audioContext.state === 'closed') {
      console.log('Audio context was closed, creating new one...');
      this.audioContext = new AudioContext();
      this.instrument = null; // Need to reload instrument with new context
      this.instrumentName = null;
    }

    if (!this.instrument) {
      throw new Error('Instrument not loaded');
    }

    // Resume audio context if suspended (browser autoplay policy)
    if (this.audioContext.state === 'suspended') {
      console.log('Resuming suspended audio context...');
      await this.audioContext.resume();
      console.log('Audio context resumed, state:', this.audioContext.state);
    }

    // Verify audio context is running
    if (this.audioContext.state !== 'running') {
      console.error('Audio context is not running:', this.audioContext.state);
      throw new Error('Audio context failed to start');
    }

    this.state = 'playing';
    this.currentData = data;
    this.onProgressCallback = onProgress;
    this.onEndCallback = onEnd;
    this.startTime = this.audioContext.currentTime - this.pausedAt;
    const beatsPerSecond = data.tempo / 60;

    console.log('Tempo:', data.tempo, 'BPM');
    console.log('Beats per second:', beatsPerSecond);
    console.log('Seconds per beat:', 60 / data.tempo);
    console.log('Start time:', this.startTime);
    console.log('Current time:', this.audioContext.currentTime);
    console.log('First few notes:', data.notes.slice(0, 5));

    // Calculate total duration
    this.totalBeats = Math.max(
      ...data.notes.map(n => n.startTime + n.duration),
      0
    );
    console.log('Total beats:', this.totalBeats);

    // Schedule all notes
    let scheduledCount = 0;
    for (const note of data.notes) {
      const timeInSeconds = note.startTime / beatsPerSecond;
      const durationInSeconds = note.duration / beatsPerSecond;
      const absoluteTime = this.audioContext.currentTime + timeInSeconds - this.pausedAt;

      console.log(`Note ${scheduledCount}: MIDI ${note.midiNote}, start ${timeInSeconds.toFixed(2)}s, duration ${durationInSeconds.toFixed(2)}s, absolute ${absoluteTime.toFixed(2)}s`);

      // Only schedule notes that haven't passed yet
      if (absoluteTime > this.audioContext.currentTime) {
        const noteName = midiToNoteName(note.midiNote);
        console.log(`Scheduling note ${note.midiNote} (${noteName}) at ${absoluteTime.toFixed(3)} with duration ${durationInSeconds.toFixed(3)}`);
        try {
          // soundfont-player API: play(note, when, options)
          // Try both MIDI number and note name
          const result = this.instrument.play(
            noteName, // Use note name like "C4" instead of MIDI number
            absoluteTime,
            {
              duration: durationInSeconds,
              gain: 1.0  // Ensure full volume
            }
          );
          console.log('Play result:', result);
          scheduledCount++;
        } catch (err) {
          console.error('Failed to play note:', err);
        }
      } else {
        console.log(`Skipping note ${note.midiNote} (time passed)`);
      }
    }
    console.log(`Total notes scheduled: ${scheduledCount}/${data.notes.length}`);

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
