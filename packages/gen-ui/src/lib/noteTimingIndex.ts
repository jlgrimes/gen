import type { PlaybackNote } from '../types';

interface IndexedNote {
  note: PlaybackNote;
  endTime: number; // startTime + duration
}

/**
 * Efficiently maps beat positions to active notes using binary search.
 * Provides O(log n + k) lookup where k is the number of active notes.
 */
export class NoteTimingIndex {
  private notesByStartTime: IndexedNote[];
  private activeNotesCache: Map<number, Set<PlaybackNote>>;

  constructor(notes: PlaybackNote[]) {
    // Sort notes by start time for binary search
    this.notesByStartTime = notes
      .map(note => ({
        note,
        endTime: note.startTime + note.duration,
      }))
      .sort((a, b) => a.note.startTime - b.note.startTime);

    this.activeNotesCache = new Map();
  }

  /**
   * Returns all notes that are active (playing) at the given beat position.
   * A note is active if: startTime <= beatPosition < endTime
   */
  getActiveNotes(beatPosition: number): PlaybackNote[] {
    // Check cache first
    const cached = this.activeNotesCache.get(beatPosition);
    if (cached) {
      return Array.from(cached);
    }

    const activeNotes: PlaybackNote[] = [];

    // Binary search to find the first note that could be active
    let left = 0;
    let right = this.notesByStartTime.length;

    while (left < right) {
      const mid = Math.floor((left + right) / 2);
      if (this.notesByStartTime[mid].note.startTime <= beatPosition) {
        left = mid + 1;
      } else {
        right = mid;
      }
    }

    // Scan backwards to find all active notes
    // Start from the last note that starts at or before beatPosition
    for (let i = left - 1; i >= 0; i--) {
      const indexed = this.notesByStartTime[i];
      const { note, endTime } = indexed;

      // If note hasn't started yet, we're done
      if (note.startTime > beatPosition) {
        break;
      }

      // If note has ended, skip it
      if (endTime <= beatPosition) {
        continue;
      }

      // Note is active
      activeNotes.push(note);
    }

    // Cache the result
    this.activeNotesCache.set(beatPosition, new Set(activeNotes));

    return activeNotes;
  }

  /**
   * Pre-compute active notes for specific timestamps.
   * Useful for optimizing repeated queries at known positions.
   */
  precomputeForTimestamps(timestamps: number[]): void {
    for (const timestamp of timestamps) {
      this.getActiveNotes(timestamp);
    }
  }

  /**
   * Clear the cache. Call this if you need to free memory.
   */
  clearCache(): void {
    this.activeNotesCache.clear();
  }
}
