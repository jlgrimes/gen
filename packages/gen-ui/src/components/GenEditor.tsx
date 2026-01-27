import { useEffect, useRef, useCallback, useMemo } from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine, placeholder as cmPlaceholder } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { lintGutter, setDiagnostics, Diagnostic } from "@codemirror/lint";
import { createModPointGutter, modPointsState, setModPointMarkers } from "./ModPointGutter";
import type { CompileError, InstrumentGroup } from "../types";

interface GenEditorProps {
  value: string;
  onChange: (value: string) => void;
  error: CompileError | null;
  placeholder?: string;
  instrumentGroup?: InstrumentGroup;
  modPointsForGroup?: Map<number, number>;  // line -> shift for current group
  onModPointToggle?: (line: number, currentShift: number | null) => void;
}

const theme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "14px",
  },
  ".cm-scroller": {
    fontFamily: "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Monaco, Consolas, monospace",
    overflow: "auto",
  },
  ".cm-content": {
    padding: "12px 0",
  },
  ".cm-line": {
    padding: "0 12px",
  },
  ".cm-gutters": {
    backgroundColor: "transparent",
    borderRight: "1px solid #e5e7eb",
    color: "#9ca3af",
  },
  ".cm-lineNumbers .cm-gutterElement": {
    padding: "0 8px 0 12px",
    minWidth: "32px",
  },
  ".cm-activeLine": {
    backgroundColor: "#f9fafb",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "#f9fafb",
  },
  "&.cm-focused .cm-cursor": {
    borderLeftColor: "#374151",
  },
  "&.cm-focused .cm-selectionBackground, ::selection": {
    backgroundColor: "#dbeafe",
  },
  ".cm-lintRange-error": {
    backgroundImage: "none",
    backgroundColor: "rgba(239, 68, 68, 0.2)",
    borderBottom: "2px solid #ef4444",
  },
  // Mod point gutter styles
  ".cm-modpoints-gutter": {
    width: "20px",
  },
  ".cm-modpoints-gutter .cm-gutterElement": {
    padding: "0 2px",
    cursor: "pointer",
  },
  ".mod-point-marker": {
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    width: "16px",
    height: "16px",
    fontSize: "12px",
    fontWeight: "bold",
    borderRadius: "3px",
  },
  ".mod-point-up": {
    backgroundColor: "#3b82f6",
    color: "#fff",
  },
  ".mod-point-down": {
    backgroundColor: "#f97316",
    color: "#fff",
  },
});

const placeholderExtension = (text: string) =>
  cmPlaceholder(text);

export function GenEditor({
  value,
  onChange,
  error,
  placeholder,
  instrumentGroup,
  modPointsForGroup,
  onModPointToggle,
}: GenEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const valueRef = useRef(value);
  const onModPointToggleRef = useRef(onModPointToggle);

  // Keep the toggle callback ref up to date
  useEffect(() => {
    onModPointToggleRef.current = onModPointToggle;
  }, [onModPointToggle]);

  const updateListener = useCallback(
    () =>
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          const newValue = update.state.doc.toString();
          valueRef.current = newValue;
          onChange(newValue);
        }
      }),
    [onChange]
  );

  // Memoize the mod point gutter extension
  const modPointGutterExtension = useMemo(() => {
    if (!instrumentGroup || !onModPointToggle) return null;
    return createModPointGutter((line, currentShift) => {
      onModPointToggleRef.current?.(line, currentShift);
    });
  }, [instrumentGroup, !!onModPointToggle]);

  // Initialize editor
  useEffect(() => {
    if (!containerRef.current) return;

    const extensions = [
      lineNumbers(),
      highlightActiveLine(),
      history(),
      lintGutter(),
      keymap.of([...defaultKeymap, ...historyKeymap]),
      theme,
      placeholderExtension(placeholder ?? ""),
      updateListener(),
    ];

    // Add mod point gutter if we have an instrument group
    if (modPointGutterExtension) {
      extensions.push(modPointGutterExtension);
    }

    const state = EditorState.create({
      doc: value,
      extensions,
    });

    const view = new EditorView({
      state,
      parent: containerRef.current,
    });

    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, [modPointGutterExtension]);

  // Sync external value changes
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;

    if (value !== valueRef.current) {
      valueRef.current = value;
      view.dispatch({
        changes: {
          from: 0,
          to: view.state.doc.length,
          insert: value,
        },
      });
    }
  }, [value]);

  // Update error diagnostics
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;

    let diagnostics: Diagnostic[] = [];

    if (error && error.line !== null) {
      const line = error.line;
      const col = error.column ?? 1;

      // Get the line info from the document
      if (line >= 1 && line <= view.state.doc.lines) {
        const lineInfo = view.state.doc.line(line);
        const from = lineInfo.from + Math.min(col - 1, lineInfo.length);
        // Highlight to end of line or at least one character
        const to = Math.max(from + 1, lineInfo.to);

        diagnostics = [
          {
            from,
            to,
            severity: "error",
            message: error.message,
          },
        ];
      }
    }

    view.dispatch(setDiagnostics(view.state, diagnostics));
  }, [error]);

  // Update mod point markers when they change
  useEffect(() => {
    const view = viewRef.current;
    if (!view || !instrumentGroup) return;

    // Check if the state field exists before dispatching
    try {
      view.state.field(modPointsState);
      view.dispatch({
        effects: setModPointMarkers.of(modPointsForGroup ?? new Map()),
      });
    } catch {
      // State field not present, ignore
    }
  }, [modPointsForGroup, instrumentGroup]);

  return <div ref={containerRef} className="h-full w-full" />;
}
