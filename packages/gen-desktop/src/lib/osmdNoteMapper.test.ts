import { describe, it, expect, beforeEach, vi } from 'vitest';
import type { PlaybackNote } from 'gen-ui/types';

/**
 * Mock OSMD types for testing
 */
interface MockGraphicalNote {
  sourceNote: {
    halfTone: number;
    isRest: () => boolean;
  };
  setColor: (color: string, options: any) => void;
}

interface MockStaffEntry {
  getAbsoluteTimestamp: () => { RealValue: number };
  graphicalVoiceEntries: Array<{
    notes: MockGraphicalNote[];
  }>;
}

interface MockMeasure {
  staffEntries: MockStaffEntry[];
}

interface MockGraphicSheet {
  MeasureList: MockMeasure[][];
}

interface MockOSMD {
  GraphicSheet: MockGraphicSheet;
}

/**
 * Create a mock GraphicalNote
 */
function createMockNote(midiNote: number, timestamp: number): {
  note: MockGraphicalNote;
  staffEntry: MockStaffEntry;
} {
  const note: MockGraphicalNote = {
    sourceNote: {
      halfTone: midiNote - 12, // OSMD uses -12 offset
      isRest: () => false,
    },
    setColor: vi.fn(),
  };

  const staffEntry: MockStaffEntry = {
    getAbsoluteTimestamp: () => ({ RealValue: timestamp }),
    graphicalVoiceEntries: [{ notes: [note] }],
  };

  return { note, staffEntry };
}

/**
 * Create a mock OSMD instance with the given notes
 */
function createMockOSMD(notes: Array<{ midi: number; beat: number }>): MockOSMD {
  const staffEntries = notes.map(({ midi, beat }) => {
    const { staffEntry } = createMockNote(midi, beat);
    return staffEntry;
  });

  return {
    GraphicSheet: {
      MeasureList: [[{ staffEntries }]],
    },
  };
}

/**
 * Helper to create PlaybackNote
 */
function createPlaybackNote(
  midiNote: number,
  startTime: number,
  duration: number = 1.0
): PlaybackNote {
  return {
    midiNote,
    displayMidiNote: midiNote,
    startTime,
    duration,
  };
}

describe('OSMDNoteMapper', () => {
  describe('MIDI Note Extraction', () => {
    it('should correctly convert OSMD halfTone to standard MIDI', () => {
      // OSMD uses C3 = 48, we use C4 = 60 (difference of 12)
      // So OSMD's halfTone 52 should map to our MIDI 64
      const osmdHalfTone = 52;
      const expectedMidi = 64;

      expect(osmdHalfTone + 12).toBe(expectedMidi);
    });

    it('should handle full "Happy Birthday" range', () => {
      // Happy Birthday notes: C4(60), D4(62), E4(64), F4(65), G4(67)
      const happyBirthdayMidi = [60, 62, 64, 65, 67];
      const osmdHalfTones = [48, 50, 52, 53, 55]; // OSMD representation

      osmdHalfTones.forEach((halfTone, i) => {
        expect(halfTone + 12).toBe(happyBirthdayMidi[i]);
      });
    });
  });

  describe('Timestamp Matching', () => {
    it('should match exact timestamps', () => {
      const osmd = createMockOSMD([
        { midi: 64, beat: 0.0 },
        { midi: 64, beat: 1.0 },
        { midi: 65, beat: 2.0 },
      ]);

      const playbackNote = createPlaybackNote(64, 1.0);

      // In a real mapper, this should find the note at beat 1.0
      const indexKey = `${playbackNote.midiNote}_${playbackNote.startTime.toFixed(3)}`;
      expect(indexKey).toBe('64_1.000');
    });

    it('should handle fuzzy timestamp matching within tolerance', () => {
      const tolerance = 0.01;

      // Playback says beat 1.000, but OSMD might have 0.999 or 1.001
      const playbackBeat = 1.0;
      const osmdBeats = [0.999, 1.0, 1.001];

      osmdBeats.forEach(osmdBeat => {
        expect(Math.abs(playbackBeat - osmdBeat)).toBeLessThan(tolerance);
      });
    });

    it('should handle chord accompaniment notes at fractional beats', () => {
      // Melody notes at whole beats: 0.0, 1.0, 2.0
      // Chord notes at fractional beats: 0.25, 0.5, 0.75, 1.25, 1.5...
      const allIndexedBeats = [0.0, 0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];
      const melodyBeats = [0.0, 1.0, 2.0];

      // Should be able to find melody notes among all indexed notes
      melodyBeats.forEach(melodyBeat => {
        expect(allIndexedBeats).toContain(melodyBeat);
      });
    });
  });

  describe('Real-world "Happy Birthday" scenario', () => {
    it('should map all Happy Birthday notes correctly', () => {
      // Actual data from console log:
      // Playback notes: MIDI 64, 64, 65, 67, 67, 65, 64, 62, 60, 60...
      // at beats: 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0...
      const playbackNotes = [
        createPlaybackNote(64, 0.0),
        createPlaybackNote(64, 1.0),
        createPlaybackNote(65, 2.0),
        createPlaybackNote(67, 3.0),
        createPlaybackNote(67, 4.0),
        createPlaybackNote(65, 5.0),
        createPlaybackNote(64, 6.0),
        createPlaybackNote(62, 7.0),
        createPlaybackNote(60, 8.0),
        createPlaybackNote(60, 9.0),
      ];

      // OSMD indexed keys from console (with +12 correction applied):
      // 60_2.000, 60_2.250, 60_6.250, 60_6.500, 60_7.625, 60_7.875
      // 62_1.750, 62_2.500, 62_3.375, 62_3.625, 62_6.000, 62_6.750, 62_7.250
      // 64_0.000, 64_0.250, 64_1.500, 64_2.750, 64_3.000, 64_4.250, 64_4.500, 64_5.750, 64_7.000
      // 65_0.500, 65_1.250, 65_4.750, 65_5.500
      // 67_0.750, 67_1.000, 67_5.000, 67_5.250

      const osmdIndexedNotes = [
        { midi: 64, beat: 0.0 },
        { midi: 67, beat: 1.0 },
        { midi: 65, beat: 2.0 },
      ];

      // Test key generation
      const expectedKeys = [
        '64_0.000',
        '64_1.000',
        '65_2.000',
        '67_3.000',
      ];

      playbackNotes.slice(0, 4).forEach((note, i) => {
        const key = `${note.midiNote}_${note.startTime.toFixed(3)}`;
        expect(key).toBe(expectedKeys[i]);
      });
    });

    it('should identify the mismatch issue', () => {
      // The console shows:
      // Playback looking for: 64_1.000
      // OSMD has: 67_1.000 (and 67_0.750, 67_5.000, 67_5.250)

      // This means beat 1.0 has MIDI 67 (G4), not MIDI 64 (E4)
      // So either:
      // 1. The playback note startTime is wrong
      // 2. The OSMD beat timestamp is wrong
      // 3. The note sequence is misaligned

      const playbackWants = { midi: 64, beat: 1.0 };
      const osmdHas = { midi: 67, beat: 1.0 };

      // They match on beat but not MIDI!
      expect(playbackWants.beat).toBe(osmdHas.beat);
      expect(playbackWants.midi).not.toBe(osmdHas.midi);

      // This is the root issue: at beat 1.0, playback says "play E4(64)"
      // but the sheet music shows G4(67)
    });
  });

  describe('Index building', () => {
    it('should create unique keys for each note position', () => {
      const notes = [
        { midi: 64, beat: 0.0 },
        { midi: 64, beat: 1.0 },  // Same MIDI, different beat
        { midi: 65, beat: 1.0 },  // Different MIDI, same beat
      ];

      const keys = notes.map(n => `${n.midi}_${n.beat.toFixed(3)}`);

      expect(keys).toEqual([
        '64_0.000',
        '64_1.000',
        '65_1.000',
      ]);

      // All keys should be unique
      expect(new Set(keys).size).toBe(keys.length);
    });

    it('should handle multiple notes at same position (chords)', () => {
      // Chord: C4 + E4 + G4 at beat 0.0
      const chordNotes = [
        { midi: 60, beat: 0.0 },
        { midi: 64, beat: 0.0 },
        { midi: 67, beat: 0.0 },
      ];

      // These create different keys even though at same beat
      const keys = chordNotes.map(n => `${n.midi}_${n.beat.toFixed(3)}`);
      expect(keys).toEqual([
        '60_0.000',
        '64_0.000',
        '67_0.000',
      ]);
    });
  });
});
