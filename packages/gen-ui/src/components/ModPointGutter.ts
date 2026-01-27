import { gutter, GutterMarker } from '@codemirror/view';
import { StateField, StateEffect } from '@codemirror/state';
import type { Extension } from '@codemirror/state';

// State effect for updating mod points display
export const setModPointMarkers = StateEffect.define<Map<number, number>>();

// Marker class for gutter display
class ModPointMarker extends GutterMarker {
  constructor(private shift: number | null) {
    super();
  }

  toDOM() {
    const span = document.createElement('span');
    if (this.shift !== null) {
      span.className = `mod-point-marker ${this.shift > 0 ? 'mod-point-up' : 'mod-point-down'}`;
      span.textContent = this.shift > 0 ? '^' : '_';
      span.title = this.shift > 0 ? 'Up one octave' : 'Down one octave';
    } else {
      // Empty clickable element for lines without mod points
      span.className = 'mod-point-empty';
      span.title = 'Click to add octave shift';
    }
    return span;
  }

  eq(other: ModPointMarker) {
    return this.shift === other.shift;
  }
}

// State field tracking mod points per line (line number -> shift)
export const modPointsState = StateField.define<Map<number, number>>({
  create() {
    return new Map();
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setModPointMarkers)) {
        return effect.value;
      }
    }
    return value;
  },
});

// Create the gutter extension
export function createModPointGutter(
  onToggle: (line: number, currentShift: number | null) => void
): Extension {
  return [
    modPointsState,
    gutter({
      class: 'cm-modpoints-gutter',
      lineMarker(view, line) {
        const lineNum = view.state.doc.lineAt(line.from).number;
        const points = view.state.field(modPointsState);
        const shift = points.get(lineNum);
        // Always return a marker (empty or with shift) so clicks work on all lines
        return new ModPointMarker(shift ?? null);
      },
      lineMarkerChange(update) {
        return (
          update.state.field(modPointsState) !==
          update.startState.field(modPointsState)
        );
      },
      domEventHandlers: {
        click(view, line) {
          const lineNum = view.state.doc.lineAt(line.from).number;
          const points = view.state.field(modPointsState);
          const currentShift = points.get(lineNum) ?? null;
          onToggle(lineNum, currentShift);
          return true;
        },
      },
    }),
  ];
}
