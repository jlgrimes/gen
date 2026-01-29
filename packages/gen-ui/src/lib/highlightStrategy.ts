import type { GraphicalNote } from 'opensheetmusicdisplay';

interface HighlightState {
  graphicalNote: GraphicalNote;
  originalColor?: string;
  metadata?: any;
}

/**
 * Abstract base class for note highlighting strategies.
 * Allows pluggable visual effects for playback highlighting.
 */
export abstract class HighlightStrategy {
  protected activeHighlights: Map<GraphicalNote, HighlightState> = new Map();

  /**
   * Apply highlighting to a single note.
   */
  abstract apply(note: GraphicalNote): void;

  /**
   * Remove highlighting from a single note.
   */
  abstract remove(note: GraphicalNote): void;

  /**
   * Apply highlighting to multiple notes (batch operation).
   */
  applyToNotes(notes: GraphicalNote[]): void {
    for (const note of notes) {
      this.apply(note);
    }
  }

  /**
   * Remove highlighting from multiple notes (batch operation).
   */
  removeFromNotes(notes: GraphicalNote[]): void {
    for (const note of notes) {
      this.remove(note);
    }
  }

  /**
   * Clear all active highlights.
   */
  clearAll(): void {
    const notes = Array.from(this.activeHighlights.keys());
    this.removeFromNotes(notes);
    this.activeHighlights.clear();
  }
}

/**
 * Simple color change strategy - changes note head fill color during playback.
 * This is the initial implementation that's easy to understand and modify.
 */
export class NoteheadColorStrategy extends HighlightStrategy {
  constructor(private color: string = '#007bff') {
    super();
  }

  apply(note: GraphicalNote): void {
    // Store original state if not already stored
    if (!this.activeHighlights.has(note)) {
      this.activeHighlights.set(note, {
        graphicalNote: note,
        originalColor: undefined, // OSMD doesn't expose original color easily
      });
    }

    // Apply highlight color to note head only
    note.setColor(this.color, {
      applyToNoteheads: true,
      applyToStem: false,
      applyToBeams: false,
      applyToFlag: false,
    });
  }

  remove(note: GraphicalNote): void {
    const state = this.activeHighlights.get(note);
    if (!state) return;

    // Restore to default black color
    note.setColor('#000000', {
      applyToNoteheads: true,
      applyToStem: false,
      applyToBeams: false,
      applyToFlag: false,
    });

    this.activeHighlights.delete(note);
  }

  /**
   * Change the highlight color.
   * Note: This will only affect newly highlighted notes.
   * Call clearAll() and re-highlight to update existing highlights.
   */
  setColor(color: string): void {
    this.color = color;
  }
}
