import type { OpenSheetMusicDisplay } from 'opensheetmusicdisplay';
import type { PlaybackChord } from '../types';

// Type for GraphicalChordSymbolContainer since it's not in the public OSMD types
interface GraphicalChordSymbolContainer {
  GraphicalLabel: {
    SVGNode: Node | undefined;
  };
}

// Type for GraphicalStaffEntry with chord containers
interface GraphicalStaffEntryWithChords {
  graphicalChordContainers: GraphicalChordSymbolContainer[];
  getAbsoluteTimestamp(): { RealValue: number };
}

/**
 * Extracts timestamp (in beats) from OSMD's GraphicalStaffEntry.
 * OSMD uses divisions as the unit (where divisions per quarter note varies),
 * but we need quarter notes as beats.
 * In 4/4 time with divisions=1, OSMD beat 0.25 = our beat 1.0 (multiply by 4).
 */
function extractTimestamp(staffEntry: GraphicalStaffEntryWithChords): number {
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
 * Maps PlaybackChords to OSMD's GraphicalChordSymbolContainers.
 * Builds an index of the rendered chord symbols for efficient lookups.
 */
export class OSMDChordMapper {
  // Map from timestamp key to chord containers at that position
  private chordMap: Map<string, GraphicalChordSymbolContainer[]> = new Map();
  private isIndexed: boolean = false;

  constructor(private osmd: OpenSheetMusicDisplay) {}

  /**
   * Build the index by traversing OSMD's data structure.
   * Call this after the score is rendered.
   */
  buildIndex(): void {
    this.chordMap.clear();

    const graphicSheet = this.osmd.GraphicSheet;
    if (!graphicSheet || !graphicSheet.MeasureList) {
      console.warn('OSMDChordMapper: No graphic data available');
      return;
    }

    // Traverse: MeasureList -> GraphicalMeasure -> staffEntries -> graphicalChordContainers
    for (const measureList of graphicSheet.MeasureList) {
      for (const measure of measureList) {
        if (!measure) continue;

        for (const staffEntry of measure.staffEntries) {
          const staffEntryWithChords = staffEntry as unknown as GraphicalStaffEntryWithChords;
          const chordContainers = staffEntryWithChords.graphicalChordContainers;

          if (!chordContainers || chordContainers.length === 0) continue;

          const timestamp = extractTimestamp(staffEntryWithChords);
          const key = this.makeKey(timestamp);

          // Store in map (handle potential collisions with array)
          if (!this.chordMap.has(key)) {
            this.chordMap.set(key, []);
          }
          this.chordMap.get(key)!.push(...chordContainers);
        }
      }
    }

    this.isIndexed = true;
    console.log('[OSMDChordMapper] Built index with', this.chordMap.size, 'chord positions');
    console.log('[OSMDChordMapper] Chord timestamps:', Array.from(this.chordMap.keys()));
  }

  /**
   * Find the GraphicalChordSymbolContainers at the given timestamp.
   * Returns an empty array if no match is found.
   */
  findChordContainersAtTime(timestamp: number): GraphicalChordSymbolContainer[] {
    if (!this.isIndexed) {
      console.warn('OSMDChordMapper: Index not built. Call buildIndex() first.');
      return [];
    }

    // Try exact match first
    const exactKey = this.makeKey(timestamp);
    const exactMatches = this.chordMap.get(exactKey);
    if (exactMatches && exactMatches.length > 0) {
      return exactMatches;
    }

    // Try fuzzy match with tolerance
    for (const [key, chordContainers] of Array.from(this.chordMap.entries())) {
      const keyTimestamp = parseFloat(key);
      if (matchesTimestamp(timestamp, keyTimestamp)) {
        return chordContainers;
      }
    }

    return [];
  }

  /**
   * Find chord containers for multiple PlaybackChords.
   * Returns all matching containers.
   */
  findChordContainers(playbackChords: PlaybackChord[]): GraphicalChordSymbolContainer[] {
    const result: GraphicalChordSymbolContainer[] = [];
    const seenTimestamps = new Set<string>();

    for (const playbackChord of playbackChords) {
      // Use osmdTimestamp for visual matching, not startTime (which is for audio)
      const key = this.makeKey(playbackChord.osmdTimestamp);
      // Avoid duplicate lookups for chords at the same timestamp
      if (seenTimestamps.has(key)) continue;
      seenTimestamps.add(key);

      const containers = this.findChordContainersAtTime(playbackChord.osmdTimestamp);
      result.push(...containers);
    }

    return result;
  }

  /**
   * Get the SVG nodes for the given chord containers.
   * Filters out containers without valid SVG nodes.
   */
  getSvgNodes(containers: GraphicalChordSymbolContainer[]): SVGElement[] {
    const nodes: SVGElement[] = [];

    for (const container of containers) {
      const svgNode = container.GraphicalLabel?.SVGNode;
      if (svgNode && svgNode instanceof SVGElement) {
        nodes.push(svgNode);
      }
    }

    return nodes;
  }

  /**
   * Creates a key for the chord map based on timestamp.
   */
  private makeKey(timestamp: number): string {
    return timestamp.toFixed(3);
  }

  /**
   * Check if the index has been built.
   */
  get indexed(): boolean {
    return this.isIndexed;
  }

  /**
   * Get the number of indexed chord positions.
   */
  get size(): number {
    return this.chordMap.size;
  }
}
