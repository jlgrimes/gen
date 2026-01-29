import { describe, it, expect, beforeEach } from 'vitest';
import { OpenSheetMusicDisplay } from 'opensheetmusicdisplay';
import { OSMDNoteMapper } from './osmdNoteMapper';
import type { PlaybackData } from '../types';

/**
 * Test to understand how triplets are represented in OSMD vs our playback data
 */

const TRIPLET_MUSICXML = `<?xml version="1.0" encoding="UTF-8"?>
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
          <step>C</step>
          <octave>4</octave>
        </pitch>
        <duration>1</duration>
        <type>quarter</type>
      </note>
      <note>
        <pitch>
          <step>D</step>
          <octave>4</octave>
        </pitch>
        <duration>0.666667</duration>
        <type>eighth</type>
        <time-modification>
          <actual-notes>3</actual-notes>
          <normal-notes>2</normal-notes>
        </time-modification>
        <notations>
          <tuplet type="start"/>
        </notations>
      </note>
      <note>
        <pitch>
          <step>E</step>
          <octave>4</octave>
        </pitch>
        <duration>0.666667</duration>
        <type>eighth</type>
        <time-modification>
          <actual-notes>3</actual-notes>
          <normal-notes>2</normal-notes>
        </time-modification>
      </note>
      <note>
        <pitch>
          <step>F</step>
          <octave>4</octave>
        </pitch>
        <duration>0.666667</duration>
        <type>eighth</type>
        <time-modification>
          <actual-notes>3</actual-notes>
          <normal-notes>2</normal-notes>
        </time-modification>
        <notations>
          <tuplet type="stop"/>
        </notations>
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

describe('Triplet Highlighting', () => {
  let container: HTMLDivElement;
  let osmd: OpenSheetMusicDisplay;

  beforeEach(async () => {
    container = document.createElement('div');
    document.body.appendChild(container);

    osmd = new OpenSheetMusicDisplay(container, {
      autoResize: false,
      backend: 'svg',
      drawTitle: false,
    });

    await osmd.load(TRIPLET_MUSICXML);
    await osmd.render();
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  it('should show how OSMD indexes triplet notes', () => {
    const mapper = new OSMDNoteMapper(osmd);
    mapper.buildIndex();

    console.log('\n=== OSMD Triplet Index ===');
    console.log('Total indexed notes:', mapper.size);

    // Try to understand what beats OSMD uses for triplets
    // Sequence: C (quarter), D E F (eighth triplet), G (quarter)
    // Expected beats: 0, 1, 1.333, 1.667, 2
    // But with our *4 multiplier, it might be: 0, 4, 5.333, 6.667, 8

    const playbackNotes = [
      { midiNote: 60, displayMidiNote: 60, startTime: 0.0, duration: 1.0 },     // C
      { midiNote: 62, displayMidiNote: 62, startTime: 1.0, duration: 0.6667 },  // D
      { midiNote: 64, displayMidiNote: 64, startTime: 1.6667, duration: 0.6667 }, // E
      { midiNote: 65, displayMidiNote: 65, startTime: 2.3333, duration: 0.6667 }, // F
      { midiNote: 67, displayMidiNote: 67, startTime: 3.0, duration: 1.0 },     // G
    ];

    console.log('\n=== Testing Playback Notes ===');
    playbackNotes.forEach((note, i) => {
      const found = mapper.findGraphicalNote(note);
      console.log(`Note ${i} (MIDI ${note.midiNote} at ${note.startTime.toFixed(4)}): ${found ? 'FOUND' : 'NOT FOUND'}`);
    });
  });

  it('should reveal the actual OSMD timestamps for triplets', () => {
    const mapper = new OSMDNoteMapper(osmd);

    // Temporarily expose the internal map for debugging
    (mapper as any).buildIndex();
    const noteMap = (mapper as any).noteMap;

    console.log('\n=== All OSMD Keys (with triplets) ===');
    const allKeys = Array.from(noteMap.keys()).sort();
    allKeys.forEach(key => {
      const [midi, beat] = key.split('_');
      console.log(`  ${key} (MIDI ${midi}, beat ${beat})`);
    });

    // This will show us the actual beat values OSMD uses
    expect(noteMap.size).toBeGreaterThan(0);
  });

  it('should show the OSMD structure for triplets', () => {
    const graphicSheet = osmd.GraphicSheet;

    console.log('\n=== OSMD Structure Analysis ===');
    let noteCount = 0;

    for (let mli = 0; mli < graphicSheet.MeasureList.length; mli++) {
      const measureList = graphicSheet.MeasureList[mli];

      for (let mi = 0; mi < measureList.length; mi++) {
        const measure = measureList[mi];
        if (!measure) continue;

        console.log(`\nMeasure ${mi}: ${measure.staffEntries.length} staff entries`);

        for (let sei = 0; sei < measure.staffEntries.length; sei++) {
          const staffEntry = measure.staffEntries[sei];
          const timestamp = staffEntry.getAbsoluteTimestamp();

          console.log(`  StaffEntry[${sei}]: timestamp=${timestamp.RealValue * 4}`);

          for (const voiceEntry of staffEntry.graphicalVoiceEntries) {
            console.log(`    VoiceEntry: ${voiceEntry.notes.length} notes`);

            for (const note of voiceEntry.notes) {
              if (note.sourceNote && !note.sourceNote.isRest()) {
                const midi = note.sourceNote.halfTone + 12;
                noteCount++;

                console.log(`      Note: MIDI ${midi}, halfTone=${note.sourceNote.halfTone}`);

                // Check if the note itself has timing info
                if (note.sourceNote.length) {
                  console.log(`        Note.length: ${note.sourceNote.length.RealValue}`);
                }
              }
            }
          }
        }
      }
    }

    console.log(`\nTotal notes found: ${noteCount}`);
    expect(noteCount).toBe(5); // C, D, E, F, G
  });
});
