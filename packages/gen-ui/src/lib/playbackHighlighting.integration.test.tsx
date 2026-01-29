import { describe, it, expect, beforeEach, vi } from 'vitest';
import { OpenSheetMusicDisplay } from 'opensheetmusicdisplay';
import { OSMDNoteMapper } from './osmdNoteMapper';
import { NoteTimingIndex } from './noteTimingIndex';
import { NoteheadColorStrategy } from './highlightStrategy';
import { PlaybackHighlightController } from './playbackHighlightController';
import type { PlaybackData, PlaybackNote } from '../types';

/**
 * Integration test for the full note highlighting system
 * Tests: NoteTimingIndex + OSMDNoteMapper + HighlightStrategy + PlaybackHighlightController
 */

const ODE_TO_JOY_MUSICXML = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 3.1 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="3.1">
  <part-list>
    <score-part id="P1">
      <part-name>Music</part-name>
    </score-part>
  </part-list>
  <part id="P1">
    <measure number="1">
      <attributes>
        <divisions>1</divisions>
        <key>
          <fifths>0</fifths>
        </key>
        <time>
          <beats>4</beats>
          <beat-type>4</beat-type>
        </time>
        <clef>
          <sign>G</sign>
          <line>2</line>
        </clef>
      </attributes>
      <note>
        <pitch>
          <step>E</step>
          <octave>4</octave>
        </pitch>
        <duration>1</duration>
        <type>quarter</type>
      </note>
      <note>
        <pitch>
          <step>E</step>
          <octave>4</octave>
        </pitch>
        <duration>1</duration>
        <type>quarter</type>
      </note>
      <note>
        <pitch>
          <step>F</step>
          <octave>4</octave>
        </pitch>
        <duration>1</duration>
        <type>quarter</type>
      </note>
      <note>
        <pitch>
          <step>G</step>
          <octave>4</octave>
        </pitch>
        <duration>1</duration>
        <type>quarter</type>
      </note>
    </measure>
  </part>
</score-partwise>`;

describe('Playback Highlighting Integration', () => {
  let container: HTMLDivElement;
  let osmd: OpenSheetMusicDisplay;

  beforeEach(async () => {
    // Create container for OSMD
    container = document.createElement('div');
    document.body.appendChild(container);

    // Initialize OSMD
    osmd = new OpenSheetMusicDisplay(container, {
      autoResize: false,
      backend: 'svg',
      drawTitle: false,
    });

    // Load Ode to Joy
    await osmd.load(ODE_TO_JOY_MUSICXML);
    await osmd.render();
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  it('should build index with correct MIDI notes (E=64, F=65, G=67)', () => {
    const mapper = new OSMDNoteMapper(osmd);
    mapper.buildIndex();

    expect(mapper.indexed).toBe(true);
    expect(mapper.size).toBeGreaterThan(0);

    // The index should contain E4(64), F4(65), G4(67) at beats 0, 1, 2, 3
    const playbackNotes: PlaybackNote[] = [
      { midiNote: 64, displayMidiNote: 64, startTime: 0.0, duration: 1.0 },
      { midiNote: 64, displayMidiNote: 64, startTime: 1.0, duration: 1.0 },
      { midiNote: 65, displayMidiNote: 65, startTime: 2.0, duration: 1.0 },
      { midiNote: 67, displayMidiNote: 67, startTime: 3.0, duration: 1.0 },
    ];

    const results = playbackNotes.map(note => ({
      key: `${note.midiNote}_${note.startTime.toFixed(3)}`,
      found: mapper.findGraphicalNote(note) !== null,
    }));

    console.log('Mapper results:', results);

    // At least the first note should be found
    expect(results[0].found).toBe(true);

    // If first note works but others don't, this test will show it
    const failedNotes = results.filter(r => !r.found);
    if (failedNotes.length > 0) {
      console.error('Failed to find notes:', failedNotes);
    }
  });

  it('should highlight first note, then second note sequentially', async () => {
    const playbackData: PlaybackData = {
      tempo: 120,
      notes: [
        { midiNote: 64, displayMidiNote: 64, startTime: 0.0, duration: 1.0 },
        { midiNote: 64, displayMidiNote: 64, startTime: 1.0, duration: 1.0 },
        { midiNote: 65, displayMidiNote: 65, startTime: 2.0, duration: 1.0 },
        { midiNote: 67, displayMidiNote: 67, startTime: 3.0, duration: 1.0 },
      ],
      chords: [],
    };

    const strategy = new NoteheadColorStrategy('#007bff');
    const controller = new PlaybackHighlightController(playbackData, osmd, strategy);

    // Track what gets highlighted/unhighlighted
    const highlightLog: string[] = [];
    const originalApply = strategy.apply.bind(strategy);
    const originalRemove = strategy.remove.bind(strategy);

    strategy.apply = vi.fn((note) => {
      highlightLog.push(`apply:${note.sourceNote.halfTone + 12}`);
      originalApply(note);
    });

    strategy.remove = vi.fn((note) => {
      highlightLog.push(`remove:${note.sourceNote.halfTone + 12}`);
      originalRemove(note);
    });

    // Beat 0.0: First note (E4, MIDI 64) should highlight
    controller.updateHighlight(0.0);
    expect(highlightLog).toContain('apply:64');

    // Beat 1.0: Second note (E4, MIDI 64) should highlight, first should unhighlight
    highlightLog.length = 0; // Clear log
    controller.updateHighlight(1.0);

    console.log('Highlight log at beat 1.0:', highlightLog);

    // CRITICAL TEST: At beat 1.0, we should:
    // 1. Remove highlight from first note
    // 2. Apply highlight to second note
    // If only first note ever works, we'll see no remove/apply here
    expect(highlightLog.length).toBeGreaterThan(0);

    // Beat 2.0: Third note (F4, MIDI 65) should highlight
    highlightLog.length = 0;
    controller.updateHighlight(2.0);

    console.log('Highlight log at beat 2.0:', highlightLog);
    expect(highlightLog.length).toBeGreaterThan(0);
  });

  it('should find all 4 notes in Ode to Joy', () => {
    const playbackData: PlaybackData = {
      tempo: 120,
      notes: [
        { midiNote: 64, displayMidiNote: 64, startTime: 0.0, duration: 1.0 }, // E
        { midiNote: 64, displayMidiNote: 64, startTime: 1.0, duration: 1.0 }, // E
        { midiNote: 65, displayMidiNote: 65, startTime: 2.0, duration: 1.0 }, // F
        { midiNote: 67, displayMidiNote: 67, startTime: 3.0, duration: 1.0 }, // G
      ],
      chords: [],
    };

    const mapper = new OSMDNoteMapper(osmd);
    mapper.buildIndex();

    const foundNotes = playbackData.notes.map((note, i) => {
      const graphicalNote = mapper.findGraphicalNote(note);
      return {
        index: i,
        midi: note.midiNote,
        beat: note.startTime,
        found: graphicalNote !== null,
      };
    });

    console.log('Found notes:', foundNotes);

    // All notes should be found
    foundNotes.forEach((result, i) => {
      expect(result.found, `Note ${i} (MIDI ${result.midi} at beat ${result.beat}) should be found`).toBe(true);
    });
  });

  it('should identify the exact failure point', () => {
    const mapper = new OSMDNoteMapper(osmd);
    mapper.buildIndex();

    const timingIndex = new NoteTimingIndex([
      { midiNote: 64, displayMidiNote: 64, startTime: 0.0, duration: 1.0 },
      { midiNote: 64, displayMidiNote: 64, startTime: 1.0, duration: 1.0 },
    ]);

    // Test each component separately
    console.log('\n=== Testing NoteTimingIndex ===');
    const activeAt0 = timingIndex.getActiveNotes(0.0);
    const activeAt1 = timingIndex.getActiveNotes(1.0);

    console.log('Active at beat 0:', activeAt0.map(n => `${n.midiNote}_${n.startTime}`));
    console.log('Active at beat 1:', activeAt1.map(n => `${n.midiNote}_${n.startTime}`));

    expect(activeAt0.length).toBe(1);
    expect(activeAt1.length).toBe(1);
    expect(activeAt0[0].startTime).toBe(0.0);
    expect(activeAt1[0].startTime).toBe(1.0);

    console.log('\n=== Testing OSMDNoteMapper ===');
    const note0 = mapper.findGraphicalNote(activeAt0[0]);
    const note1 = mapper.findGraphicalNote(activeAt1[0]);

    console.log('Found note at beat 0:', note0 !== null);
    console.log('Found note at beat 1:', note1 !== null);

    // This will show us exactly where it fails
    if (note0 === null) {
      throw new Error('FAILURE: Cannot find note at beat 0 in OSMD');
    }
    if (note1 === null) {
      throw new Error('FAILURE: Cannot find note at beat 1 in OSMD');
    }
  });
});
