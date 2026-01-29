import type { PlaybackNote } from '../types';

/**
 * Single source of truth for note timing calculations.
 * Both playback scheduling and visual highlighting use this engine.
 */
export class NoteTimingEngine {
  private notes: PlaybackNote[];
  private notesByMidiAndOrder: Map<string, PlaybackNote[]>;

  constructor(notes: PlaybackNote[]) {
    this.notes = notes;
    this.notesByMidiAndOrder = new Map();

    // Index notes by MIDI number for quick lookup
    // Store them in order of startTime
    for (const note of notes) {
      const key = note.midiNote.toString();
      if (!this.notesByMidiAndOrder.has(key)) {
        this.notesByMidiAndOrder.set(key, []);
      }
      this.notesByMidiAndOrder.get(key)!.push(note);
    }
  }

  /**
   * Get all notes, in order.
   * This is the single source of truth for what notes exist and when they play.
   */
  getAllNotes(): PlaybackNote[] {
    return this.notes;
  }

  /**
   * Find a note by MIDI number and approximate beat position.
   * Returns the closest note within tolerance.
   *
   * This is used by the OSMD mapper to match graphical notes to playback notes.
   */
  findNoteByMidiAndBeat(midiNote: number, beat: number, tolerance: number = 0.5): PlaybackNote | null {
    const candidates = this.notesByMidiAndOrder.get(midiNote.toString()) || [];

    // Find the note with the closest startTime to the given beat
    let closest: PlaybackNote | null = null;
    let closestDistance = Infinity;

    for (const note of candidates) {
      const distance = Math.abs(note.startTime - beat);
      if (distance < tolerance && distance < closestDistance) {
        closest = note;
        closestDistance = distance;
      }
    }

    return closest;
  }

  /**
   * Get notes that are active (playing) at a given beat position.
   * A note is active if: startTime <= beat < startTime + duration
   */
  getActiveNotesAtBeat(beat: number): PlaybackNote[] {
    return this.notes.filter(note =>
      note.startTime <= beat && beat < note.startTime + note.duration
    );
  }
}
