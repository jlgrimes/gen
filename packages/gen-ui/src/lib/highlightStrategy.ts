import type { GraphicalNote } from 'opensheetmusicdisplay';

interface HighlightState {
  graphicalNote: GraphicalNote;
  originalColor?: string;
  metadata?: any;
}

interface ChordHighlightState {
  svgElement: SVGElement;
  originalFill?: string;
}

/**
 * Abstract base class for note highlighting strategies.
 * Allows pluggable visual effects for playback highlighting.
 */
export abstract class HighlightStrategy {
  protected activeHighlights: Map<GraphicalNote, HighlightState> = new Map();
  protected activeChordHighlights: Map<SVGElement, ChordHighlightState> = new Map();

  /**
   * Apply highlighting to a single note.
   */
  abstract apply(note: GraphicalNote): void;

  /**
   * Remove highlighting from a single note.
   */
  abstract remove(note: GraphicalNote): void;

  /**
   * Apply highlighting to a chord symbol SVG element.
   */
  abstract applyToChord(svgElement: SVGElement): void;

  /**
   * Remove highlighting from a chord symbol SVG element.
   */
  abstract removeFromChord(svgElement: SVGElement): void;

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
   * Apply highlighting to multiple chord symbols (batch operation).
   */
  applyToChords(svgElements: SVGElement[]): void {
    for (const svg of svgElements) {
      this.applyToChord(svg);
    }
  }

  /**
   * Remove highlighting from multiple chord symbols (batch operation).
   */
  removeFromChords(svgElements: SVGElement[]): void {
    for (const svg of svgElements) {
      this.removeFromChord(svg);
    }
  }

  /**
   * Clear all active highlights (notes and chords).
   */
  clearAll(): void {
    const notes = Array.from(this.activeHighlights.keys());
    this.removeFromNotes(notes);
    this.activeHighlights.clear();

    const chords = Array.from(this.activeChordHighlights.keys());
    this.removeFromChords(chords);
    this.activeChordHighlights.clear();
  }
}

/**
 * Simple color change strategy - changes note head fill color during playback.
 * Also highlights chord symbols when they are being played.
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

  applyToChord(svgElement: SVGElement): void {
    // Store original state if not already stored
    if (!this.activeChordHighlights.has(svgElement)) {
      // Get current fill color - chord symbols are typically text elements
      const currentFill = svgElement.getAttribute('fill') ||
                         window.getComputedStyle(svgElement).fill ||
                         '#000000';

      this.activeChordHighlights.set(svgElement, {
        svgElement,
        originalFill: currentFill,
      });
    }

    // Apply highlight color to the chord symbol
    // For text elements, we set the fill attribute
    svgElement.setAttribute('fill', this.color);

    // Also set style.fill to ensure it takes effect
    if (svgElement instanceof SVGTextElement || svgElement.tagName === 'text') {
      svgElement.style.fill = this.color;
    }

    // If it's a group element, apply to all child text elements
    const textChildren = svgElement.querySelectorAll('text');
    textChildren.forEach(text => {
      text.setAttribute('fill', this.color);
      text.style.fill = this.color;
    });
  }

  removeFromChord(svgElement: SVGElement): void {
    const state = this.activeChordHighlights.get(svgElement);
    if (!state) return;

    const originalFill = state.originalFill || '#000000';

    // Restore original fill color
    svgElement.setAttribute('fill', originalFill);

    if (svgElement instanceof SVGTextElement || svgElement.tagName === 'text') {
      svgElement.style.fill = originalFill;
    }

    // Restore child text elements
    const textChildren = svgElement.querySelectorAll('text');
    textChildren.forEach(text => {
      text.setAttribute('fill', originalFill);
      text.style.fill = originalFill;
    });

    this.activeChordHighlights.delete(svgElement);
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
