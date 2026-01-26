import { useState, useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import {
  OpenSheetMusicDisplay,
  TransposeCalculator,
  MusicSheetCalculator,
} from 'opensheetmusicdisplay';
import { jsPDF } from 'jspdf';
import 'svg2pdf.js';
import { Download } from 'lucide-react';
import { Sidebar, type ScoreInfo } from '@/components/ui/sidebar';
import { GenEditor, type CompileError } from '@/components/GenEditor';
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from '@/components/ui/resizable';

// Transposition options with halftone values for OSMD
const TRANSPOSE_OPTIONS = [
  { label: 'C (Concert Pitch)', halftones: 0 },
  { label: 'Bb (Clarinet, Tenor Sax, Trumpet)', halftones: 2 }, // Up a major 2nd
  { label: 'Eb (Alto, Baritone Sax)', halftones: 9 }, // Up a major 6th
  { label: 'F (French Horn)', halftones: 7 }, // Up a perfect 5th
] as const;

// Initialize OSMD's transpose calculator
MusicSheetCalculator.transposeCalculator = new TransposeCalculator();

interface CompileResult {
  status: 'success' | 'error';
  xml?: string;
  error?: CompileError;
}

function App() {
  const [genSource, setGenSource] = useState('');
  const [scores, setScores] = useState<ScoreInfo[]>([]);
  const [selectedScore, setSelectedScore] = useState<string | null>(null);
  const [error, setError] = useState<CompileError | null>(null);
  const [isCompiling, setIsCompiling] = useState(false);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [transposeIndex, setTransposeIndex] = useState(0); // Index into TRANSPOSE_OPTIONS
  const sheetMusicRef = useRef<HTMLDivElement>(null);
  const osmdRef = useRef<OpenSheetMusicDisplay | null>(null);
  const debounceRef = useRef<NodeJS.Timeout | null>(null);

  const toggleSidebar = useCallback(() => {
    setIsSidebarCollapsed(prev => !prev);
  }, []);

  // Load embedded scores on mount
  useEffect(() => {
    const loadScores = async () => {
      try {
        const scoreList = await invoke<ScoreInfo[]>('list_scores');
        setScores(scoreList);
        // Auto-select first score if available
        if (scoreList.length > 0) {
          setGenSource(scoreList[0].content);
          setSelectedScore(scoreList[0].name);
        }
      } catch (e) {
        console.error('Failed to load scores:', e);
      }
    };
    loadScores();
  }, []);

  const compileAndRender = useCallback(
    async (source: string, halftones: number) => {
      if (!source.trim()) return;

      setIsCompiling(true);
      try {
        // Compile without transposition - OSMD handles it
        const result = await invoke<CompileResult>('compile_gen_unchecked', {
          source,
        });

        if (result.status === 'error' && result.error) {
          setError(result.error);
          return;
        }

        setError(null);

        if (result.xml && sheetMusicRef.current) {
          if (!osmdRef.current) {
            osmdRef.current = new OpenSheetMusicDisplay(sheetMusicRef.current, {
              autoResize: true,
              backend: 'svg',
            });
          }
          await osmdRef.current.load(result.xml);

          // Set transposition on the sheet
          if (osmdRef.current.Sheet) {
            osmdRef.current.Sheet.Transpose = halftones;
            osmdRef.current.updateGraphic();
          }

          osmdRef.current.render();
        }
      } catch (e) {
        setError({ message: String(e), line: null, column: null });
      } finally {
        setIsCompiling(false);
      }
    },
    [],
  );

  // Debounced compile on source change or transpose change
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      compileAndRender(genSource, TRANSPOSE_OPTIONS[transposeIndex].halftones);
    }, 150);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [genSource, transposeIndex, compileAndRender]);

  const handleScoreSelect = (score: ScoreInfo) => {
    setGenSource(score.content);
    setSelectedScore(score.name);
  };

  const exportToPdf = useCallback(async () => {
    const svg = sheetMusicRef.current?.querySelector('svg');
    if (!svg) return;

    // Show save dialog
    const baseName = selectedScore?.replace(/\.gen$/, '') || 'score';
    const filePath = await save({
      defaultPath: `${baseName}.pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    });

    if (!filePath) return; // User cancelled

    const svgWidth = svg.clientWidth || svg.getBoundingClientRect().width;
    const svgHeight = svg.clientHeight || svg.getBoundingClientRect().height;

    const pdf = new jsPDF({
      orientation: svgWidth > svgHeight ? 'landscape' : 'portrait',
      unit: 'pt',
      format: [svgWidth, svgHeight],
    });

    await pdf.svg(svg, { width: svgWidth, height: svgHeight });

    // Get PDF as array buffer and write to file
    const pdfData = pdf.output('arraybuffer');
    await writeFile(filePath, new Uint8Array(pdfData));
  }, [selectedScore]);

  return (
    <div className='flex h-screen w-screen'>
      {/* Sidebar - outside of resizable panels */}
      <Sidebar
        scores={scores}
        onScoreSelect={handleScoreSelect}
        selectedScore={selectedScore}
        isCollapsed={isSidebarCollapsed}
        onToggleCollapse={toggleSidebar}
      />

      {/* Main content area with resizable panels */}
      <ResizablePanelGroup orientation='horizontal' className='flex-1 h-full'>
        {/* Editor Panel */}
        <ResizablePanel defaultSize={35} minSize={20}>
          <div className='h-full border-r border-border flex flex-col bg-white'>
            <div className='p-3 border-b border-border flex items-center justify-between'>
              <h2 className='font-semibold text-sm'>
                {selectedScore || 'Editor'}
                {isCompiling && (
                  <span className='ml-2 text-xs text-muted-foreground'>
                    compiling...
                  </span>
                )}
              </h2>
            </div>
            <div className='flex-1 overflow-hidden'>
              <GenEditor
                value={genSource}
                onChange={setGenSource}
                error={error}
                placeholder='Select a score or start typing...'
              />
            </div>
            {error && (
              <div className='p-3 text-sm text-red-600 border-t border-border bg-red-50 max-h-24 overflow-auto'>
                {error.line !== null ? `Line ${error.line}: ` : ''}
                {error.message}
              </div>
            )}
          </div>
        </ResizablePanel>

        <ResizableHandle />

        {/* Sheet Music Panel */}
        <ResizablePanel defaultSize={65} minSize={30}>
          <div className='h-full flex flex-col bg-white min-w-0'>
            <div className='p-3 border-b border-border flex items-center justify-between'>
              <h2 className='font-semibold text-sm'>Sheet Music</h2>
              <button
                onClick={exportToPdf}
                className='flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors'
                title='Export to PDF'
              >
                <Download size={14} />
                Export PDF
              </button>
            </div>
            {/* Transpose Toolbar */}
            <div className='px-4 py-2 border-b border-border bg-gray-50 flex items-center gap-3'>
              <label className='text-xs font-medium text-gray-600'>
                Transpose:
              </label>
              <select
                value={transposeIndex}
                onChange={e => setTransposeIndex(Number(e.target.value))}
                className='px-2 py-1 text-xs border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
              >
                {TRANSPOSE_OPTIONS.map((option, index) => (
                  <option key={option.label} value={index}>
                    {option.label}
                  </option>
                ))}
              </select>
            </div>
            <div className='flex-1 overflow-auto p-4'>
              <div ref={sheetMusicRef} className='w-full' />
            </div>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}

export default App;
