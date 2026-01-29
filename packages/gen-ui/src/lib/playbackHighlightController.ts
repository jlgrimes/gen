import type { OpenSheetMusicDisplay } from 'opensheetmusicdisplay';
import type { PlaybackData, PlaybackNote } from '../types';
import { NoteTimingIndex } from './noteTimingIndex';
import { OSMDNoteMapper } from './osmdNoteMapper';
import { HighlightStrategy } from './highlightStrategy';

/**
 * Orchestrates note highlighting during playback.
 * Coordinates timing, mapping, and visual updates.
 */
export class PlaybackHighlightController {
  private timingIndex: NoteTimingIndex;
  private noteMapper: OSMDNoteMapper;
  private strategy: HighlightStrategy;
  private currentlyHighlightedNotes: Set<PlaybackNote> = new Set();

  constructor(
    playbackData: PlaybackData,
    osmd: OpenSheetMusicDisplay,
    strategy: HighlightStrategy
  ) {
    this.timingIndex = new NoteTimingIndex(playbackData.notes);
    this.noteMapper = new OSMDNoteMapper(osmd);
    this.strategy = strategy;

    // Build OSMD index
    this.noteMapper.buildIndex();
  }

  /**
   * Update highlighting based on current playback position.
   * Call this on every onProgress callback from PlaybackEngine.
   */
  updateHighlight(currentBeat: number): void {
    // 1. Find which notes should be highlighted now
    const activeNotes = this.timingIndex.getActiveNotes(currentBeat);
    const activeSet = new Set(activeNotes);

    // 2. Find notes to unhighlight (were active, now finished)
    const toUnhighlight = Array.from(this.currentlyHighlightedNotes).filter(
      note => !activeSet.has(note)
    );

    // 3. Find notes to highlight (newly active)
    const toHighlight = activeNotes.filter(
      note => !this.currentlyHighlightedNotes.has(note)
    );

    // 4. Update SVG elements
    if (toUnhighlight.length > 0) {
      const graphicalNotes = this.noteMapper.findGraphicalNotes(toUnhighlight);
      if (graphicalNotes.length > 0) {
        this.strategy.removeFromNotes(graphicalNotes);
      }
    }

    if (toHighlight.length > 0) {
      const graphicalNotes = this.noteMapper.findGraphicalNotes(toHighlight);
      if (graphicalNotes.length > 0) {
        this.strategy.applyToNotes(graphicalNotes);
      }
    }

    // 5. Update state
    this.currentlyHighlightedNotes = activeSet;
  }

  /**
   * Clear all highlights and reset state.
   * Call this when playback stops or pauses.
   */
  reset(): void {
    this.strategy.clearAll();
    this.currentlyHighlightedNotes.clear();
  }

  /**
   * Swap the highlighting strategy without rebuilding the controller.
   * Useful for changing visual effects on the fly.
   */
  setStrategy(strategy: HighlightStrategy): void {
    // Clear current highlights
    this.strategy.clearAll();
    this.currentlyHighlightedNotes.clear();

    // Set new strategy
    this.strategy = strategy;
  }

  /**
   * Get the current highlight strategy.
   */
  getStrategy(): HighlightStrategy {
    return this.strategy;
  }

  /**
   * Get diagnostic information about the controller state.
   */
  getDebugInfo(): {
    indexed: boolean;
    indexSize: number;
    currentlyHighlighted: number;
  } {
    return {
      indexed: this.noteMapper.indexed,
      indexSize: this.noteMapper.size,
      currentlyHighlighted: this.currentlyHighlightedNotes.size,
    };
  }
}
