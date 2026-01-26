import { useEffect, useRef, useCallback } from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine, placeholder as cmPlaceholder } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { lintGutter, setDiagnostics, Diagnostic } from "@codemirror/lint";

export interface CompileError {
  message: string;
  line: number | null;
  column: number | null;
}

interface GenEditorProps {
  value: string;
  onChange: (value: string) => void;
  error: CompileError | null;
  placeholder?: string;
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
});

const placeholderExtension = (text: string) =>
  cmPlaceholder(text);

export function GenEditor({ value, onChange, error, placeholder }: GenEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const valueRef = useRef(value);

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

  // Initialize editor
  useEffect(() => {
    if (!containerRef.current) return;

    const state = EditorState.create({
      doc: value,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        history(),
        lintGutter(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        theme,
        placeholderExtension(placeholder ?? ""),
        updateListener(),
      ],
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
  }, []);

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

  return <div ref={containerRef} className="h-full w-full" />;
}
