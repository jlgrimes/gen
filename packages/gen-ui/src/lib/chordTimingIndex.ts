import type { PlaybackChord } from '../types';

interface IndexedChord {
  chord: PlaybackChord;
  endTime: number; // startTime + duration
}

/**
 * Efficiently maps beat positions to active chords using binary search.
 * Provides O(log n + k) lookup where k is the number of active chords.
 */
export class ChordTimingIndex {
  private chordsByStartTime: IndexedChord[];

  constructor(chords: PlaybackChord[]) {
    // Sort chords by start time for binary search
    this.chordsByStartTime = chords
      .map(chord => ({
        chord,
        endTime: chord.startTime + chord.duration,
      }))
      .sort((a, b) => a.chord.startTime - b.chord.startTime);
  }

  /**
   * Returns all chords that are active (playing) at the given beat position.
   * A chord is active if: startTime <= beatPosition < endTime
   */
  getActiveChords(beatPosition: number): PlaybackChord[] {
    const activeChords: PlaybackChord[] = [];

    // Binary search to find the first chord that could be active
    let left = 0;
    let right = this.chordsByStartTime.length;

    while (left < right) {
      const mid = Math.floor((left + right) / 2);
      if (this.chordsByStartTime[mid].chord.startTime <= beatPosition) {
        left = mid + 1;
      } else {
        right = mid;
      }
    }

    // Scan backwards to find all active chords
    // Start from the last chord that starts at or before beatPosition
    for (let i = left - 1; i >= 0; i--) {
      const indexed = this.chordsByStartTime[i];
      const { chord, endTime } = indexed;

      // If chord hasn't started yet, we're done
      if (chord.startTime > beatPosition) {
        break;
      }

      // If chord has ended, skip it
      if (endTime <= beatPosition) {
        continue;
      }

      // Chord is active
      activeChords.push(chord);
    }

    return activeChords;
  }

  /**
   * Get the number of indexed chords.
   */
  get size(): number {
    return this.chordsByStartTime.length;
  }
}
