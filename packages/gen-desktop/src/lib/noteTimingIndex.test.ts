import { describe, it, expect } from 'vitest';
import type { PlaybackNote } from 'gen-ui/types';

/**
 * Mock implementation of NoteTimingIndex for testing
 */
class NoteTimingIndex {
  private notesByStartTime: Array<{ note: PlaybackNote; endTime: number }>;

  constructor(notes: PlaybackNote[]) {
    this.notesByStartTime = notes
      .map(note => ({
        note,
        endTime: note.startTime + note.duration,
      }))
      .sort((a, b) => a.note.startTime - b.note.startTime);
  }

  getActiveNotes(beatPosition: number): PlaybackNote[] {
    const active: PlaybackNote[] = [];

    for (const { note, endTime } of this.notesByStartTime) {
      // Note is active if: startTime <= beatPosition < endTime
      if (note.startTime <= beatPosition && beatPosition < endTime) {
        active.push(note);
      }

      // Early exit if we've passed all possible active notes
      if (note.startTime > beatPosition) {
        break;
      }
    }

    return active;
  }
}

function createNote(midi: number, start: number, duration: number = 1.0): PlaybackNote {
  return {
    midiNote: midi,
    displayMidiNote: midi,
    startTime: start,
    duration,
  };
}

describe('NoteTimingIndex', () => {
  describe('Active note detection', () => {
    it('should find note at exact start time', () => {
      const notes = [
        createNote(64, 0.0, 1.0),
        createNote(64, 1.0, 1.0),
      ];

      const index = new NoteTimingIndex(notes);

      // At beat 0.0, first note should be active
      const activeAt0 = index.getActiveNotes(0.0);
      expect(activeAt0).toHaveLength(1);
      expect(activeAt0[0].startTime).toBe(0.0);

      // At beat 1.0, second note should be active (not first!)
      const activeAt1 = index.getActiveNotes(1.0);
      expect(activeAt1).toHaveLength(1);
      expect(activeAt1[0].startTime).toBe(1.0);
    });

    it('should find note during its duration', () => {
      const notes = [createNote(64, 0.0, 2.0)]; // 2-beat note

      const index = new NoteTimingIndex(notes);

      // Should be active at start
      expect(index.getActiveNotes(0.0)).toHaveLength(1);

      // Should be active in middle
      expect(index.getActiveNotes(0.5)).toHaveLength(1);
      expect(index.getActiveNotes(1.0)).toHaveLength(1);
      expect(index.getActiveNotes(1.5)).toHaveLength(1);

      // Should NOT be active at end (beat 2.0)
      expect(index.getActiveNotes(2.0)).toHaveLength(0);
    });

    it('should handle Ode to Joy sequence', () => {
      // E E F G (all quarter notes)
      const notes = [
        createNote(64, 0.0, 1.0), // E
        createNote(64, 1.0, 1.0), // E
        createNote(65, 2.0, 1.0), // F
        createNote(67, 3.0, 1.0), // G
      ];

      const index = new NoteTimingIndex(notes);

      // At beat 0.0, first E should be active
      const at0 = index.getActiveNotes(0.0);
      expect(at0).toHaveLength(1);
      expect(at0[0].midiNote).toBe(64);
      expect(at0[0].startTime).toBe(0.0);

      // At beat 1.0, second E should be active (NOT first E!)
      const at1 = index.getActiveNotes(1.0);
      expect(at1).toHaveLength(1);
      expect(at1[0].midiNote).toBe(64);
      expect(at1[0].startTime).toBe(1.0);

      // At beat 2.0, F should be active
      const at2 = index.getActiveNotes(2.0);
      expect(at2).toHaveLength(1);
      expect(at2[0].midiNote).toBe(65);
      expect(at2[0].startTime).toBe(2.0);

      // At beat 3.0, G should be active
      const at3 = index.getActiveNotes(3.0);
      expect(at3).toHaveLength(1);
      expect(at3[0].midiNote).toBe(67);
      expect(at3[0].startTime).toBe(3.0);
    });

    it('should handle note boundaries correctly', () => {
      // Quarter note at beat 0
      const notes = [createNote(60, 0.0, 1.0)];
      const index = new NoteTimingIndex(notes);

      // Active at start
      expect(index.getActiveNotes(0.0)).toHaveLength(1);

      // Active just before end
      expect(index.getActiveNotes(0.99)).toHaveLength(1);

      // NOT active at end (beat 1.0)
      expect(index.getActiveNotes(1.0)).toHaveLength(0);

      // NOT active after end
      expect(index.getActiveNotes(1.01)).toHaveLength(0);
    });

    it('should handle overlapping notes (chords)', () => {
      // Two notes starting at the same time
      const notes = [
        createNote(60, 0.0, 1.0), // C
        createNote(64, 0.0, 1.0), // E
      ];

      const index = new NoteTimingIndex(notes);

      // Both should be active at beat 0
      const active = index.getActiveNotes(0.0);
      expect(active).toHaveLength(2);
    });

    it('should handle no active notes', () => {
      const notes = [
        createNote(60, 0.0, 1.0),
        createNote(62, 2.0, 1.0),
      ];

      const index = new NoteTimingIndex(notes);

      // At beat 1.0, no notes should be active (gap between notes)
      expect(index.getActiveNotes(1.0)).toHaveLength(0);

      // Before any notes
      expect(index.getActiveNotes(-1.0)).toHaveLength(0);

      // After all notes
      expect(index.getActiveNotes(10.0)).toHaveLength(0);
    });
  });

  describe('Edge cases', () => {
    it('should handle very short notes (sixteenth notes)', () => {
      // Sixteenth note = 0.25 beats
      const notes = [createNote(60, 0.0, 0.25)];
      const index = new NoteTimingIndex(notes);

      expect(index.getActiveNotes(0.0)).toHaveLength(1);
      expect(index.getActiveNotes(0.1)).toHaveLength(1);
      expect(index.getActiveNotes(0.24)).toHaveLength(1);
      expect(index.getActiveNotes(0.25)).toHaveLength(0); // Ended
    });

    it('should handle long sustained notes', () => {
      // Whole note = 4 beats
      const notes = [createNote(60, 0.0, 4.0)];
      const index = new NoteTimingIndex(notes);

      expect(index.getActiveNotes(0.0)).toHaveLength(1);
      expect(index.getActiveNotes(1.0)).toHaveLength(1);
      expect(index.getActiveNotes(2.0)).toHaveLength(1);
      expect(index.getActiveNotes(3.0)).toHaveLength(1);
      expect(index.getActiveNotes(3.99)).toHaveLength(1);
      expect(index.getActiveNotes(4.0)).toHaveLength(0);
    });

    it('should handle floating point precision', () => {
      // Eighth note = 0.5 beats
      const notes = [
        createNote(60, 0.0, 0.5),
        createNote(62, 0.5, 0.5),
        createNote(64, 1.0, 0.5),
      ];

      const index = new NoteTimingIndex(notes);

      expect(index.getActiveNotes(0.0)).toHaveLength(1);
      expect(index.getActiveNotes(0.5)).toHaveLength(1);
      expect(index.getActiveNotes(1.0)).toHaveLength(1);
    });
  });

  describe('Real-world scenario: first note works, others dont', () => {
    it('should identify if the issue is in timing index', () => {
      // Ode to Joy: E E F G
      const notes = [
        createNote(64, 0.0, 1.0),
        createNote(64, 1.0, 1.0),
        createNote(65, 2.0, 1.0),
        createNote(67, 3.0, 1.0),
      ];

      const index = new NoteTimingIndex(notes);

      // Test at each beat during playback
      const testBeats = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5];
      const results = testBeats.map(beat => ({
        beat,
        activeCount: index.getActiveNotes(beat).length,
        active: index.getActiveNotes(beat),
      }));

      // Log for debugging
      console.log('Active notes at each beat:', results);

      // Verify each note becomes active at its start time
      expect(index.getActiveNotes(0.0)[0].startTime).toBe(0.0);
      expect(index.getActiveNotes(1.0)[0].startTime).toBe(1.0);
      expect(index.getActiveNotes(2.0)[0].startTime).toBe(2.0);
      expect(index.getActiveNotes(3.0)[0].startTime).toBe(3.0);

      // If first note works but others don't, the issue is NOT in NoteTimingIndex
    });
  });
});
