import { useEffect, useRef } from 'react';
import * as monaco from 'monaco-editor';
import { registerGenLanguage } from 'gen-lang-support/monarch';
import type { CompileError, InstrumentGroup } from '../types';

// Register Gen language once
let languageRegistered = false;
function ensureLanguageRegistered() {
  if (languageRegistered) return;
  registerGenLanguage(monaco);
  languageRegistered = true;
}

interface GenMonacoEditorProps {
  value: string;
  onChange: (value: string) => void;
  error: CompileError | null;
  placeholder?: string;
  instrumentGroup?: InstrumentGroup;
  modPointsForGroup?: Map<number, number>;
  onModPointToggle?: (line: number, currentShift: number | null) => void;
}

export function GenMonacoEditor({
  value,
  onChange,
  error,
  placeholder: _placeholder,
  instrumentGroup: _instrumentGroup,
  modPointsForGroup: _modPointsForGroup,
  onModPointToggle: _onModPointToggle,
}: GenMonacoEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const decorationsRef = useRef<monaco.editor.IEditorDecorationsCollection | null>(null);
  const valueRef = useRef(value);
  const isInternalChange = useRef(false);

  // Initialize editor
  useEffect(() => {
    if (!containerRef.current) return;

    ensureLanguageRegistered();

    const editor = monaco.editor.create(containerRef.current, {
      value,
      language: 'gen',
      theme: 'vs',
      fontSize: 14,
      fontFamily: "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Monaco, Consolas, monospace",
      lineNumbers: 'on',
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      automaticLayout: true,
      wordWrap: 'off',
      lineHeight: 20,
      padding: { top: 12, bottom: 12 },
      renderLineHighlight: 'line',
      selectionHighlight: true,
      occurrencesHighlight: 'off',
      folding: false,
      glyphMargin: false,
      lineDecorationsWidth: 12,
      lineNumbersMinChars: 3,
      overviewRulerBorder: false,
      scrollbar: {
        verticalScrollbarSize: 10,
        horizontalScrollbarSize: 10,
      },
    });

    editorRef.current = editor;
    decorationsRef.current = editor.createDecorationsCollection([]);

    // Listen for changes
    editor.onDidChangeModelContent(() => {
      if (isInternalChange.current) return;
      const newValue = editor.getValue();
      valueRef.current = newValue;
      onChange(newValue);
    });

    return () => {
      editor.dispose();
      editorRef.current = null;
    };
  }, []);

  // Sync external value changes
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor) return;

    if (value !== valueRef.current) {
      valueRef.current = value;
      isInternalChange.current = true;
      editor.setValue(value);
      isInternalChange.current = false;
    }
  }, [value]);

  // Update error decorations
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor || !decorationsRef.current) return;

    const model = editor.getModel();
    if (!model) return;

    if (error && error.line !== null) {
      const line = error.line;
      const col = error.column ?? 1;

      if (line >= 1 && line <= model.getLineCount()) {
        const lineLength = model.getLineLength(line);
        const startCol = Math.min(col, lineLength + 1);
        const endCol = lineLength + 1;

        // Set error marker
        monaco.editor.setModelMarkers(model, 'gen', [{
          severity: monaco.MarkerSeverity.Error,
          message: error.message,
          startLineNumber: line,
          startColumn: startCol,
          endLineNumber: line,
          endColumn: endCol,
        }]);

        // Add line decoration
        decorationsRef.current.set([{
          range: new monaco.Range(line, 1, line, 1),
          options: {
            isWholeLine: true,
            className: 'gen-error-line',
          },
        }]);
      }
    } else {
      // Clear markers and decorations
      monaco.editor.setModelMarkers(model, 'gen', []);
      decorationsRef.current.set([]);
    }
  }, [error]);

  return (
    <div
      ref={containerRef}
      className="h-full w-full"
    />
  );
}
