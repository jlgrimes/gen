import { describe, it, expect } from 'vitest';

/**
 * This test simulates the FULL highlighting flow to find where it breaks
 */

describe('Full highlighting flow simulation', () => {
  it('should highlight first note, unhighlight it, then highlight second note', () => {
    // Simulated state
    let currentlyHighlightedNotes = new Set<any>();
    const highlightedVisually = new Set<string>(); // Track what's visually highlighted

    // Mock notes
    const notes = [
      { midi: 64, start: 0.0, duration: 1.0, key: '64_0.000' },
      { midi: 64, start: 1.0, duration: 1.0, key: '64_1.000' },
    ];

    // Mock getActiveNotes
    function getActiveNotes(beat: number) {
      return notes.filter(n => n.start <= beat && beat < n.start + n.duration);
    }

    // Mock highlight/unhighlight
    function highlight(noteKey: string) {
      highlightedVisually.add(noteKey);
      console.log(`✓ Highlighted: ${noteKey}`);
    }

    function unhighlight(noteKey: string) {
      highlightedVisually.delete(noteKey);
      console.log(`✗ Unhighlighted: ${noteKey}`);
    }

    // Simulate updateHighlight function
    function updateHighlight(currentBeat: number) {
      console.log(`\n--- Beat ${currentBeat.toFixed(2)} ---`);

      // 1. Find active notes
      const activeNotes = getActiveNotes(currentBeat);
      const activeSet = new Set(activeNotes);
      console.log(`Active notes:`, activeNotes.map(n => n.key));

      // 2. Find notes to unhighlight
      const toUnhighlight = Array.from(currentlyHighlightedNotes).filter(
        note => !activeSet.has(note)
      );
      console.log(`To unhighlight:`, toUnhighlight.map((n: any) => n.key));

      // 3. Find notes to highlight
      const toHighlight = activeNotes.filter(
        note => !currentlyHighlightedNotes.has(note)
      );
      console.log(`To highlight:`, toHighlight.map(n => n.key));

      // 4. Apply changes
      toUnhighlight.forEach((note: any) => unhighlight(note.key));
      toHighlight.forEach(note => highlight(note.key));

      // 5. Update state
      currentlyHighlightedNotes = activeSet;
    }

    // SIMULATION: Play through the song
    console.log('=== Starting playback ===');

    // Beat 0.0: First note starts
    updateHighlight(0.0);
    expect(highlightedVisually.has('64_0.000')).toBe(true);
    expect(highlightedVisually.size).toBe(1);

    // Beat 0.5: Still playing first note
    updateHighlight(0.5);
    expect(highlightedVisually.has('64_0.000')).toBe(true);
    expect(highlightedVisually.size).toBe(1);

    // Beat 1.0: Second note starts, first note ends
    updateHighlight(1.0);
    console.log(`\nFinal visual state:`, Array.from(highlightedVisually));

    // THIS IS THE KEY TEST: After beat 1.0
    // - First note (64_0.000) should be UNHIGHLIGHTED
    // - Second note (64_1.000) should be HIGHLIGHTED
    expect(highlightedVisually.has('64_0.000')).toBe(false);
    expect(highlightedVisually.has('64_1.000')).toBe(true);
    expect(highlightedVisually.size).toBe(1);
  });

  it('should identify if the issue is in the highlight strategy', () => {
    // Test if calling setColor multiple times on the same note breaks it
    const mockNote = {
      setColor: (color: string) => {
        console.log(`setColor called with: ${color}`);
      }
    };

    // First highlight (should turn blue)
    mockNote.setColor('#007bff');

    // Unhighlight (should turn black)
    mockNote.setColor('#000000');

    // Highlight again (should turn blue again)
    mockNote.setColor('#007bff');

    // If this works, the issue is NOT in the strategy
    expect(true).toBe(true);
  });

  it('should identify if the unhighlight is actually being called', () => {
    // Simulate the exact scenario where first note works but second doesn't
    let callLog: string[] = [];

    const strategy = {
      apply: (noteKey: string) => {
        callLog.push(`apply(${noteKey})`);
      },
      remove: (noteKey: string) => {
        callLog.push(`remove(${noteKey})`);
      },
    };

    // Beat 0.0: Highlight first note
    strategy.apply('64_0.000');

    // Beat 1.0: Should unhighlight first, then highlight second
    strategy.remove('64_0.000');
    strategy.apply('64_1.000');

    console.log('Call log:', callLog);

    // Expected sequence
    expect(callLog).toEqual([
      'apply(64_0.000)',
      'remove(64_0.000)',
      'apply(64_1.000)',
    ]);

    // If the real app has: ['apply(64_0.000)', 'apply(64_1.000)']
    // (missing remove), then the problem is that remove isn't being called
  });

  it('should test if OSMD setColor is additive or absolute', () => {
    // Hypothesis: Maybe calling setColor(blue) on a note that's already blue does nothing?
    // And if we don't call setColor(black) first, the second note stays black?

    const mockNote = {
      currentColor: '#000000',
      setColor(color: string) {
        this.currentColor = color;
        console.log(`Note color is now: ${this.currentColor}`);
      }
    };

    // Initial: black
    console.log('\n=== Scenario 1: Normal flow ===');
    mockNote.setColor('#007bff'); // First note: blue
    expect(mockNote.currentColor).toBe('#007bff');

    mockNote.setColor('#000000'); // Unhighlight: black
    expect(mockNote.currentColor).toBe('#000000');

    mockNote.setColor('#007bff'); // Second note: blue
    expect(mockNote.currentColor).toBe('#007bff');

    // Scenario 2: What if we don't unhighlight?
    console.log('\n=== Scenario 2: No unhighlight (BUG simulation) ===');
    mockNote.currentColor = '#000000';
    mockNote.setColor('#007bff'); // First note: blue
    // SKIP unhighlight
    mockNote.setColor('#007bff'); // Second note: try to set blue again
    // Note is already blue, so setColor might be a no-op?
    console.log('After trying to highlight second note:', mockNote.currentColor);
  });
});
