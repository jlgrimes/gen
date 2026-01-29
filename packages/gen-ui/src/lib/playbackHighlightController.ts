import type { OpenSheetMusicDisplay } from 'opensheetmusicdisplay';
import type { PlaybackData, PlaybackNote } from '../types';
import { NoteTimingIndex } from './noteTimingIndex';
import { ChordTimingIndex } from './chordTimingIndex';
import { OSMDNoteMapper } from './osmdNoteMapper';
import { OSMDChordMapper } from './osmdChordMapper';
import { HighlightStrategy } from './highlightStrategy';

/**
 * Orchestrates note and chord highlighting during playback.
 * Coordinates timing, mapping, and visual updates.
 */
export class PlaybackHighlightController {
  private noteTimingIndex: NoteTimingIndex;
  private chordTimingIndex: ChordTimingIndex;
  private noteMapper: OSMDNoteMapper;
  private chordMapper: OSMDChordMapper;
  private strategy: HighlightStrategy;
  private currentlyHighlightedNotes: Set<PlaybackNote> = new Set();
  private currentlyHighlightedChordTimestamps: Set<string> = new Set();

  constructor(
    playbackData: PlaybackData,
    osmd: OpenSheetMusicDisplay,
    strategy: HighlightStrategy
  ) {
    this.noteTimingIndex = new NoteTimingIndex(playbackData.notes);
    this.chordTimingIndex = new ChordTimingIndex(playbackData.chords || []);
    this.noteMapper = new OSMDNoteMapper(osmd);
    this.chordMapper = new OSMDChordMapper(osmd);
    this.strategy = strategy;

    // DEBUG: Log all osmdMatchKey values from Rust
    console.log('[PlaybackHighlightController] Rust osmdMatchKeys:',
      playbackData.notes.map(n => n.osmdMatchKey));
    console.log('[PlaybackHighlightController] Chords count:', playbackData.chords?.length || 0);
    console.log('[PlaybackHighlightController] Chord osmdTimestamps from Rust:',
      playbackData.chords?.map(c => c.osmdTimestamp.toFixed(3)) || []);

    // Build OSMD indices
    this.noteMapper.buildIndex();
    this.chordMapper.buildIndex();
  }

  /**
   * Update highlighting based on current playback position.
   * Call this on every onProgress callback from PlaybackEngine.
   */
  updateHighlight(currentBeat: number): void {
    // Update note highlighting
    this.updateNoteHighlight(currentBeat);

    // Update chord highlighting
    this.updateChordHighlight(currentBeat);
  }

  /**
   * Update note highlighting based on current beat.
   */
  private updateNoteHighlight(currentBeat: number): void {
    // 1. Find which notes should be highlighted now
    const activeNotes = this.noteTimingIndex.getActiveNotes(currentBeat);
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
   * Update chord highlighting based on current beat.
   */
  private updateChordHighlight(currentBeat: number): void {
    // 1. Find which chords should be highlighted now
    const activeChords = this.chordTimingIndex.getActiveChords(currentBeat);

    // Create a set of OSMD timestamp keys for active chords (for visual matching)
    const activeTimestampSet = new Set(
      activeChords.map(chord => chord.osmdTimestamp.toFixed(3))
    );

    // 2. Find chords to unhighlight (were active, now finished)
    const toUnhighlightTimestamps = Array.from(this.currentlyHighlightedChordTimestamps).filter(
      timestamp => !activeTimestampSet.has(timestamp)
    );

    // 3. Find chords to highlight (newly active)
    const toHighlightChords = activeChords.filter(
      chord => !this.currentlyHighlightedChordTimestamps.has(chord.osmdTimestamp.toFixed(3))
    );

    // 4. Update SVG elements for chords being unhighlighted
    if (toUnhighlightTimestamps.length > 0) {
      for (const timestamp of toUnhighlightTimestamps) {
        const containers = this.chordMapper.findChordContainersAtTime(parseFloat(timestamp));
        const svgNodes = this.chordMapper.getSvgNodes(containers);
        if (svgNodes.length > 0) {
          this.strategy.removeFromChords(svgNodes);
        }
      }
    }

    // 5. Update SVG elements for chords being highlighted
    if (toHighlightChords.length > 0) {
      const containers = this.chordMapper.findChordContainers(toHighlightChords);
      const svgNodes = this.chordMapper.getSvgNodes(containers);
      if (svgNodes.length > 0) {
        this.strategy.applyToChords(svgNodes);
      }
    }

    // 6. Update state
    this.currentlyHighlightedChordTimestamps = activeTimestampSet;
  }

  /**
   * Clear all highlights and reset state.
   * Call this when playback stops or pauses.
   */
  reset(): void {
    this.strategy.clearAll();
    this.currentlyHighlightedNotes.clear();
    this.currentlyHighlightedChordTimestamps.clear();
  }

  /**
   * Swap the highlighting strategy without rebuilding the controller.
   * Useful for changing visual effects on the fly.
   */
  setStrategy(strategy: HighlightStrategy): void {
    // Clear current highlights
    this.strategy.clearAll();
    this.currentlyHighlightedNotes.clear();
    this.currentlyHighlightedChordTimestamps.clear();

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
    notesIndexed: boolean;
    notesIndexSize: number;
    chordsIndexed: boolean;
    chordsIndexSize: number;
    currentlyHighlightedNotes: number;
    currentlyHighlightedChords: number;
  } {
    return {
      notesIndexed: this.noteMapper.indexed,
      notesIndexSize: this.noteMapper.size,
      chordsIndexed: this.chordMapper.indexed,
      chordsIndexSize: this.chordMapper.size,
      currentlyHighlightedNotes: this.currentlyHighlightedNotes.size,
      currentlyHighlightedChords: this.currentlyHighlightedChordTimestamps.size,
    };
  }
}
