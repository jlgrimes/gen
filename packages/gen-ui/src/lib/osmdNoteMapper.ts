import type { OpenSheetMusicDisplay, GraphicalNote } from 'opensheetmusicdisplay';
import type { PlaybackNote } from '../types';

/**
 * Extracts display MIDI note number from OSMD's GraphicalNote.
 * This is the transposed note that matches what's shown on the sheet music.
 */
function extractMidiNote(graphicalNote: GraphicalNote): number {
  // OSMD's halfTone is 1 octave lower than standard MIDI numbering
  // Add 12 semitones to match our MIDI calculation
  return graphicalNote.sourceNote.halfTone + 12;
}

/**
 * Extracts timestamp (in beats) from OSMD's GraphicalStaffEntry.
 * OSMD uses divisions as the unit (where divisions per quarter note varies),
 * but we need quarter notes as beats.
 * In 4/4 time with divisions=1, OSMD beat 0.25 = our beat 1.0 (multiply by 4).
 */
function extractTimestamp(staffEntry: any): number {
  const fraction = staffEntry.getAbsoluteTimestamp();
  // OSMD timestamp is in "measures", convert to quarter note beats
  // In 4/4 time: 1 measure = 4 quarter notes
  return fraction.RealValue * 4;
}

/**
 * Checks if two timestamps match within tolerance.
 */
function matchesTimestamp(playbackTime: number, graphicalTime: number, tolerance: number = 0.01): boolean {
  return Math.abs(playbackTime - graphicalTime) < tolerance;
}

/**
 * Maps PlaybackNotes to OSMD's GraphicalNotes.
 * Builds an index of the rendered score for efficient lookups.
 */
export class OSMDNoteMapper {
  private noteMap: Map<string, GraphicalNote[]> = new Map();
  private isIndexed: boolean = false;

  constructor(private osmd: OpenSheetMusicDisplay) {}

  /**
   * Build the index by traversing OSMD's data structure.
   * Call this after the score is rendered.
   */
  buildIndex(): void {
    this.noteMap.clear();

    const graphicSheet = this.osmd.GraphicSheet;
    if (!graphicSheet || !graphicSheet.MeasureList) {
      console.warn('OSMDNoteMapper: No graphic data available');
      return;
    }

    // Traverse: MeasureList → GraphicalMeasure → staffEntries → voiceEntries → notes
    for (const measureList of graphicSheet.MeasureList) {
      for (const measure of measureList) {
        if (!measure) continue;

        for (const staffEntry of measure.staffEntries) {
          const timestamp = extractTimestamp(staffEntry);

          for (const voiceEntry of staffEntry.graphicalVoiceEntries) {
            for (const note of voiceEntry.notes) {
              // Skip rests
              if (!note.sourceNote || note.sourceNote.isRest()) {
                continue;
              }

              try {
                const midiNote = extractMidiNote(note);
                const key = this.makeKey(midiNote, timestamp);

                // Store in map (handle potential collisions with array)
                if (!this.noteMap.has(key)) {
                  this.noteMap.set(key, []);
                }
                this.noteMap.get(key)!.push(note);

              } catch (err) {
                console.warn('OSMDNoteMapper: Failed to extract MIDI note', err);
              }
            }
          }
        }
      }
    }

    this.isIndexed = true;
  }

  /**
   * Find the GraphicalNote corresponding to a PlaybackNote.
   * Returns null if no match is found.
   */
  findGraphicalNote(playbackNote: PlaybackNote): GraphicalNote | null {
    if (!this.isIndexed) {
      console.warn('OSMDNoteMapper: Index not built. Call buildIndex() first.');
      return null;
    }

    // Use concert pitch for matching (works for both transposed and non-transposed instruments)
    const displayMidi = playbackNote.midiNote;

    // Try exact match first
    const exactKey = this.makeKey(displayMidi, playbackNote.startTime);
    const exactMatches = this.noteMap.get(exactKey);
    if (exactMatches && exactMatches.length > 0) {
      return exactMatches[0]; // Return first match
    }

    // Try fuzzy match with tolerance
    for (const [key, graphicalNotes] of this.noteMap.entries()) {
      const [midiStr, timestampStr] = key.split('_');
      const midi = parseInt(midiStr, 10);
      const timestamp = parseFloat(timestampStr);

      if (
        midi === displayMidi &&
        matchesTimestamp(playbackNote.startTime, timestamp)
      ) {
        return graphicalNotes[0];
      }
    }

    return null;
  }

  /**
   * Find GraphicalNotes for multiple PlaybackNotes.
   * Filters out notes that couldn't be found.
   */
  findGraphicalNotes(playbackNotes: PlaybackNote[]): GraphicalNote[] {
    const result: GraphicalNote[] = [];

    for (const playbackNote of playbackNotes) {
      const graphicalNote = this.findGraphicalNote(playbackNote);
      if (graphicalNote) {
        result.push(graphicalNote);
      }
    }

    return result;
  }

  /**
   * Creates a composite key for the note map.
   */
  private makeKey(midiNote: number, timestamp: number): string {
    return `${midiNote}_${timestamp.toFixed(3)}`;
  }

  /**
   * Check if the index has been built.
   */
  get indexed(): boolean {
    return this.isIndexed;
  }

  /**
   * Get the number of indexed note positions.
   */
  get size(): number {
    return this.noteMap.size;
  }
}
