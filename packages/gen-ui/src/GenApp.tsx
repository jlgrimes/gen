import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import {
  OpenSheetMusicDisplay,
  TransposeCalculator,
  MusicSheetCalculator,
} from 'opensheetmusicdisplay';
import { jsPDF } from 'jspdf';
import 'svg2pdf.js';
import { Download, Eye, EyeOff } from 'lucide-react';
import { Sidebar } from '@/components/ui/sidebar';
import { GenMonacoEditor } from '@/editor/GenMonacoEditor';
import { useBreakpoint } from '@/hooks/useBreakpoint';
import { SideDrawer } from '@/components/ui/side-drawer';
import { MobileHeader } from '@/components/ui/mobile-header';
import { BottomTabBar } from '@/components/ui/bottom-tab-bar';
import type {
  CompilerAdapter,
  FileAdapter,
  PlaybackAdapter,
  PlaybackData,
  ScoreInfo,
  CompileError,
  InstrumentGroup,
  ModPoints,
  UrlAdapter,
} from './types';
import { PlaybackEngine } from './lib/playback';
import { PlaybackHighlightController } from './lib/playbackHighlightController';
import { NoteheadColorStrategy } from './lib/highlightStrategy';
import { Play, Pause, Square } from 'lucide-react';

// URL parameter helpers (hash-based routing)
function getUrlParams() {
  const hash = window.location.hash.slice(1); // Remove leading #
  const [path, search] = hash.split('?');
  const params = new URLSearchParams(search);
  return {
    score: path || params.get('score') || null,
    instrument: params.get('instrument') || null,
  };
}

function getInstrumentIndexById(id: string | null): number {
  if (!id) return 0;
  const index = INSTRUMENT_PRESETS.findIndex(p => p.id === id);
  return index >= 0 ? index : 0;
}

function updateUrl(score: string | null, instrumentId: string | null) {
  const path = score || '';
  const params = new URLSearchParams();
  if (instrumentId && instrumentId !== 'concert') {
    params.set('instrument', instrumentId);
  }
  const search = params.toString();
  const hash = '#' + path + (search ? `?${search}` : '');
  window.history.replaceState({}, '', hash);
}

// Mobile view tabs
type MobileTab = 'editor' | 'sheet';

// Clef options
type Clef = 'treble' | 'bass';

const CLEF_OPTIONS: { label: string; value: Clef }[] = [
  { label: 'Treble', value: 'treble' },
  { label: 'Bass', value: 'bass' },
];

// Transpose key options with halftone values for OSMD
const TRANSPOSE_OPTIONS = [
  { label: 'C', halftones: 0 },
  { label: 'Bb', halftones: 2 },
  { label: 'Eb', halftones: 9 },
  { label: 'F', halftones: 7 },
] as const;

// Octave shift options
const OCTAVE_OPTIONS = [
  { label: '-2', value: -2 },
  { label: '-1', value: -1 },
  { label: '0', value: 0 },
  { label: '+1', value: 1 },
  { label: '+2', value: 2 },
] as const;

// Instrument presets that set transpose, octave, and clef together
interface InstrumentPreset {
  id: string; // URL-friendly identifier
  label: string;
  transposeIndex: number; // Index into TRANSPOSE_OPTIONS
  octaveShift: number;
  clef: Clef;
  instrumentGroup?: InstrumentGroup; // For mod points support
}

const INSTRUMENT_PRESETS: InstrumentPreset[] = [
  {
    id: 'concert',
    label: 'Treble Clef (Concert)',
    transposeIndex: 0,
    octaveShift: 0,
    clef: 'treble',
  },
  {
    id: 'bass',
    label: 'Bass Clef (Concert)',
    transposeIndex: 0,
    octaveShift: -1,
    clef: 'bass',
  },
  {
    id: 'flute',
    label: 'C Flute/Piccolo',
    transposeIndex: 1,
    octaveShift: 1,
    clef: 'treble',
  },
  {
    id: 'bb',
    label: 'Bb Trumpet/Clarinet/Tenor Sax',
    transposeIndex: 1,
    octaveShift: 0,
    clef: 'treble',
    instrumentGroup: 'bb',
  },
  {
    id: 'eb',
    label: 'Eb Alto/Baritone Sax',
    transposeIndex: 2,
    octaveShift: 0,
    clef: 'treble',
    instrumentGroup: 'eb',
  },
  {
    id: 'f-horn',
    label: 'F French Horn',
    transposeIndex: 3,
    octaveShift: 0,
    clef: 'treble',
  },
];

// Map instrument presets to soundfont instrument names
const INSTRUMENT_TO_SOUNDFONT: Record<string, string> = {
  'concert': 'acoustic_grand_piano',
  'bass': 'acoustic_grand_piano',
  'flute': 'flute',
  'bb': 'trumpet', // For Bb Trumpet/Clarinet/Tenor Sax, default to trumpet
  'eb': 'alto_sax', // For Eb Alto/Baritone Sax
  'f-horn': 'french_horn',
};

// Parse mod points from source text
function parseModPointsFromSource(source: string): ModPoints {
  const result: ModPoints = { eb: [], bb: [] };
  const lines = source.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNum = i + 1; // 1-indexed

    // Look for @Group:modifier patterns (e.g., @Eb:^ or @Bb:_)
    const modPointRegex = /@(eb|bb):(\^|_)/gi;
    let match;
    while ((match = modPointRegex.exec(line)) !== null) {
      const group = match[1].toLowerCase() as 'eb' | 'bb';
      const shift = match[2] === '^' ? 1 : -1;
      result[group].push({ line: lineNum, octaveShift: shift });
    }
  }

  return result;
}

// Update source with a mod point change
function updateSourceWithModPoint(
  source: string,
  lineNum: number,
  group: InstrumentGroup,
  newShift: number | null,
): string {
  const lines = source.split('\n');
  const lineIndex = lineNum - 1;

  if (lineIndex < 0 || lineIndex >= lines.length) return source;

  let line = lines[lineIndex];

  // Remove existing mod point for this group (e.g., @Eb:^ or @Bb:_)
  const groupRegex = new RegExp(`\\s*@${group}:[\\^_]`, 'gi');
  line = line.replace(groupRegex, '');

  // Add new mod point if shift is not null
  if (newShift !== null) {
    const modifier = newShift > 0 ? '^' : '_';
    const groupLabel = group.charAt(0).toUpperCase() + group.slice(1);
    line = `${line.trimEnd()} @${groupLabel}:${modifier}`;
  }

  lines[lineIndex] = line;
  return lines.join('\n');
}

// Index for "Custom" option (after all presets)
const CUSTOM_PRESET_INDEX = INSTRUMENT_PRESETS.length;

// Letter size in millimeters
const LETTER_WIDTH_MM = 215.9; // 8.5 inches
const LETTER_HEIGHT_MM = 279.4; // 11 inches

// Initialize OSMD's transpose calculator
MusicSheetCalculator.transposeCalculator = new TransposeCalculator();

export interface GenAppProps {
  compiler: CompilerAdapter;
  files: FileAdapter;
  playback?: PlaybackAdapter;
  url?: UrlAdapter;
  scores: ScoreInfo[];
}

export function GenApp({ compiler, files, playback, url, scores }: GenAppProps) {
  const { isMobile, isTablet, isDesktop, isMobileOrTablet } = useBreakpoint();

  // Get initial values from URL
  const initialParams = useMemo(() => getUrlParams(), []);

  const [genSource, setGenSource] = useState('');
  const [selectedScore, setSelectedScore] = useState<string | null>(
    initialParams.score,
  );
  const [error, setError] = useState<CompileError | null>(null);
  const [isCompiling, setIsCompiling] = useState(false);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(true); // Start collapsed, managed by drawer on mobile/tablet
  const [isDrawerOpen, setIsDrawerOpen] = useState(false); // Drawer state for mobile/tablet
  const [isEditorVisible, setIsEditorVisible] = useState(true); // Toggle editor visibility
  const [mobileTab, setMobileTab] = useState<MobileTab>('sheet'); // Default to sheet view on mobile/tablet
  const [instrumentIndex, setInstrumentIndex] = useState(() =>
    getInstrumentIndexById(initialParams.instrument),
  ); // Index into INSTRUMENT_PRESETS, or CUSTOM_PRESET_INDEX for custom
  const [transposeIndex, setTransposeIndex] = useState(() => {
    // Initialize from preset if instrument was in URL
    const idx = getInstrumentIndexById(initialParams.instrument);
    if (idx < INSTRUMENT_PRESETS.length) {
      return INSTRUMENT_PRESETS[idx].transposeIndex;
    }
    return 0;
  });
  const [octaveShift, setOctaveShift] = useState(() => {
    const idx = getInstrumentIndexById(initialParams.instrument);
    if (idx < INSTRUMENT_PRESETS.length) {
      return INSTRUMENT_PRESETS[idx].octaveShift;
    }
    return 0;
  });
  const [clef, setClef] = useState<Clef>(() => {
    const idx = getInstrumentIndexById(initialParams.instrument);
    if (idx < INSTRUMENT_PRESETS.length) {
      return INSTRUMENT_PRESETS[idx].clef;
    }
    return 'treble';
  });
  const [zoom, setZoom] = useState(0.75); // Zoom level for sheet music
  const sheetMusicRef = useRef<HTMLDivElement>(null);
  const osmdRef = useRef<OpenSheetMusicDisplay | null>(null);
  const debounceRef = useRef<NodeJS.Timeout | null>(null);

  // Playback state
  const playbackEngineRef = useRef<PlaybackEngine | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentBeat, setCurrentBeat] = useState(0);
  const [totalBeats, setTotalBeats] = useState(0);
  const [playbackData, setPlaybackData] = useState<PlaybackData | null>(null);
  const [isLoadingPlayback, setIsLoadingPlayback] = useState(false);

  // Highlight controller for visual feedback during playback
  const highlightControllerRef = useRef<PlaybackHighlightController | null>(null);
  const [osmdRenderCount, setOsmdRenderCount] = useState(0);

  // Get current instrument group from preset
  const currentInstrumentGroup = useMemo((): InstrumentGroup | undefined => {
    if (instrumentIndex >= INSTRUMENT_PRESETS.length) return undefined;
    return INSTRUMENT_PRESETS[instrumentIndex].instrumentGroup;
  }, [instrumentIndex]);

  // Parse mod points from source
  const modPoints = useMemo(
    () => parseModPointsFromSource(genSource),
    [genSource],
  );

  // Get mod points for current instrument group as a Map (for the editor)
  const modPointsForGroup = useMemo((): Map<number, number> => {
    if (!currentInstrumentGroup) return new Map();
    const points = modPoints[currentInstrumentGroup];
    return new Map(points.map(p => [p.line, p.octaveShift]));
  }, [modPoints, currentInstrumentGroup]);

  // Handle mod point toggle from editor gutter
  const handleModPointToggle = useCallback(
    (line: number, currentShift: number | null) => {
      if (!currentInstrumentGroup) return;

      // Toggle: null -> +1 -> -1 -> null
      let newShift: number | null;
      if (currentShift === null) {
        newShift = 1;
      } else if (currentShift === 1) {
        newShift = -1;
      } else {
        newShift = null;
      }

      const updatedSource = updateSourceWithModPoint(
        genSource,
        line,
        currentInstrumentGroup,
        newShift,
      );
      console.log('Mod point toggle:', { line, newShift, updatedSource });
      setGenSource(updatedSource);
    },
    [genSource, currentInstrumentGroup],
  );

  const toggleSidebar = useCallback(() => {
    setIsSidebarCollapsed(prev => !prev);
  }, []);

  const handleOpenDocs = useCallback(() => {
    url?.openExternal('https://docs.gen.band');
  }, [url]);

  // Close drawer when switching to desktop
  useEffect(() => {
    if (!isMobileOrTablet) {
      setIsDrawerOpen(false);
    }
  }, [isMobileOrTablet]);

  // Load score from URL or default to first score
  useEffect(() => {
    if (scores.length === 0) return;

    // Try to find score from URL
    if (selectedScore) {
      const score = scores.find(s => s.name === selectedScore);
      if (score) {
        setGenSource(score.content);
        return;
      }
    }

    // Default to first score
    setGenSource(scores[0].content);
    setSelectedScore(scores[0].name);
  }, [scores, selectedScore]);

  // Sync URL when score or instrument changes
  useEffect(() => {
    const instrumentId =
      instrumentIndex < INSTRUMENT_PRESETS.length
        ? INSTRUMENT_PRESETS[instrumentIndex].id
        : null;
    updateUrl(selectedScore, instrumentId);
  }, [selectedScore, instrumentIndex]);

  // Initialize playback engine
  useEffect(() => {
    if (!playback) return; // Playback is optional

    if (!playbackEngineRef.current) {
      playbackEngineRef.current = new PlaybackEngine();
    }

    // Only dispose on final unmount, not on re-renders
    return () => {
      // Don't dispose here - it closes the AudioContext permanently
      // We'll dispose on window unload or when the component truly unmounts
    };
  }, [playback]);

  // Load instrument when instrument preset changes
  useEffect(() => {
    if (!playback || !playbackEngineRef.current) return;

    const currentPreset = INSTRUMENT_PRESETS[instrumentIndex];
    if (currentPreset) {
      const soundfont = INSTRUMENT_TO_SOUNDFONT[currentPreset.id] || 'acoustic_grand_piano';
      playbackEngineRef.current.loadInstrument(soundfont).catch((err: unknown) => {
        console.error('Failed to load instrument:', err);
      });
    }
  }, [playback, instrumentIndex]);

  // Generate playback data when source or settings change
  useEffect(() => {
    // Stop playback when source or settings change
    if (playbackEngineRef.current) {
      playbackEngineRef.current.stop();
      setIsPlaying(false);
      setCurrentBeat(0);
    }

    if (!playback || !genSource.trim()) {
      setPlaybackData(null);
      setTotalBeats(0);
      return;
    }

    const generateData = async () => {
      setIsLoadingPlayback(true);
      try {
        const transposeKey = TRANSPOSE_OPTIONS[transposeIndex]?.label as
          | 'C'
          | 'Bb'
          | 'Eb'
          | 'F'
          | undefined;

        const result = await playback.generatePlaybackData(genSource, {
          clef,
          octaveShift,
          instrumentGroup: currentInstrumentGroup,
          transposeKey,
        });

        if (result.status === 'success' && result.data) {
          setPlaybackData(result.data);
          const maxBeat = Math.max(
            ...result.data.notes.map(n => n.startTime + n.duration),
            0
          );
          setTotalBeats(maxBeat);
        } else {
          console.error('Playback data generation failed:', result);
        }
      } catch (err) {
        console.error('Failed to generate playback data:', err);
      } finally {
        setIsLoadingPlayback(false);
      }
    };

    generateData();
  }, [playback, genSource, clef, octaveShift, currentInstrumentGroup, transposeIndex]);

  // Playback handlers
  const handlePlay = useCallback(async () => {
    if (!playbackEngineRef.current || !playbackData) {
      return;
    }

    try {
      // Ensure instrument is loaded
      const currentPreset = INSTRUMENT_PRESETS[instrumentIndex];
      if (currentPreset) {
        const soundfont = INSTRUMENT_TO_SOUNDFONT[currentPreset.id] || 'acoustic_grand_piano';
        await playbackEngineRef.current.loadInstrument(soundfont);
      }

      await playbackEngineRef.current.play(
        playbackData,
        (beat: number) => {
          setCurrentBeat(beat);
          highlightControllerRef.current?.updateHighlight(beat);
        },
        () => setIsPlaying(false)
      );
      setIsPlaying(true);
    } catch (err: unknown) {
      console.error('Playback error:', err);
    }
  }, [playbackData, instrumentIndex]);

  const handlePause = useCallback(() => {
    playbackEngineRef.current?.pause();
    highlightControllerRef.current?.reset();
    setIsPlaying(false);
  }, []);

  const handleStop = useCallback(() => {
    playbackEngineRef.current?.stop();
    highlightControllerRef.current?.reset();
    setIsPlaying(false);
    setCurrentBeat(0);
  }, []);

  const handleSeek = useCallback(async (beat: number) => {
    if (!playbackEngineRef.current) return;
    await playbackEngineRef.current.seek(beat);
    setCurrentBeat(beat);
  }, []);

  const compileAndRender = useCallback(
    async (
      source: string,
      octave: number,
      selectedClef: Clef,
      instrumentGroup: InstrumentGroup | undefined,
      currentTransposeIndex: number,
    ) => {
      if (!source.trim()) return;

      setIsCompiling(true);
      try {
        // Compile with clef and instrument group parameters
        const transposeKey = TRANSPOSE_OPTIONS[currentTransposeIndex]?.label as
          | 'C'
          | 'Bb'
          | 'Eb'
          | 'F'
          | undefined;
        console.log('Compiling with:', {
          octave,
          instrumentGroup,
          transposeKey,
          sourceHasModPoint: source.includes('@'),
          sourceFirstLine: source.split('\n')[0],
        });
        const result = await compiler.compile(source, {
          clef: selectedClef,
          octaveShift: octave,
          instrumentGroup,
          transposeKey,
        });

        if (result.status === 'error' && result.error) {
          setError(result.error);
          return;
        }

        setError(null);

        if (result.xml && sheetMusicRef.current) {
          // Check if octave changed in XML
          const octave5Count = (result.xml.match(/<octave>5<\/octave>/g) || [])
            .length;
          const octave4Count = (result.xml.match(/<octave>4<\/octave>/g) || [])
            .length;
          console.log('XML octave counts:', {
            octave5: octave5Count,
            octave4: octave4Count,
          });

          if (!osmdRef.current) {
            osmdRef.current = new OpenSheetMusicDisplay(sheetMusicRef.current, {
              autoResize: false,
              backend: 'svg',
              pageFormat: 'Letter P',
              pageBackgroundColor: '#FFFFFF',
              drawingParameters: 'default',
            });
            // Enable chord symbol rendering
            if (osmdRef.current.EngravingRules) {
              osmdRef.current.EngravingRules.RenderChordSymbols = true;
            }
          }
          await osmdRef.current.load(result.xml);

          // Transposition is handled server-side in Rust
          // Do NOT set Sheet.Transpose here

          // Scale down the notation to fit more measures per page
          osmdRef.current.Zoom = zoom;
          osmdRef.current.render();
          setOsmdRenderCount(prev => prev + 1);
        }
      } catch (e) {
        setError({ message: String(e), line: null, column: null });
      } finally {
        setIsCompiling(false);
      }
    },
    [compiler, zoom],
  );

  // Debounced compile on source change or settings change
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      compileAndRender(
        genSource,
        octaveShift,
        clef,
        currentInstrumentGroup,
        transposeIndex,
      );
    }, 150);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [
    genSource,
    transposeIndex,
    octaveShift,
    clef,
    currentInstrumentGroup,
    compileAndRender,
  ]);

  // Update zoom without recompiling
  useEffect(() => {
    if (osmdRef.current) {
      osmdRef.current.Zoom = zoom;
      osmdRef.current.render();
      setOsmdRenderCount(prev => prev + 1);
    }
  }, [zoom]);

  // Re-render when editor visibility changes (layout change)
  useEffect(() => {
    if (osmdRef.current) {
      // Use requestAnimationFrame to wait for layout to settle
      const timeout = setTimeout(() => {
        requestAnimationFrame(() => {
          osmdRef.current?.render();
          setOsmdRenderCount(prev => prev + 1);
        });
      }, 50);
      return () => clearTimeout(timeout);
    }
  }, [isEditorVisible]);

  // Initialize highlight controller when playback data and OSMD are ready
  useEffect(() => {
    if (!playbackData || !osmdRef.current || osmdRenderCount === 0) {
      highlightControllerRef.current = null;
      return;
    }

    try {
      console.log('Initializing PlaybackHighlightController...');
      const strategy = new NoteheadColorStrategy('#007bff');
      highlightControllerRef.current = new PlaybackHighlightController(
        playbackData,
        osmdRef.current,
        strategy
      );
      console.log('PlaybackHighlightController initialized successfully');
    } catch (err) {
      console.error('Failed to initialize highlight controller:', err);
      highlightControllerRef.current = null;
    }

    return () => {
      highlightControllerRef.current?.reset();
    };
  }, [playbackData, osmdRenderCount]);

  const handleScoreSelect = useCallback((score: ScoreInfo) => {
    setGenSource(score.content);
    setSelectedScore(score.name);
    // Reset playback state when changing songs
    playbackEngineRef.current?.stop();
    highlightControllerRef.current?.reset();
    setIsPlaying(false);
    setCurrentBeat(0);
  }, []);

  const exportToPdf = useCallback(async () => {
    const svgs = sheetMusicRef.current?.querySelectorAll('svg');
    if (!svgs || svgs.length === 0) return;

    const baseName = selectedScore?.replace(/\.gen$/, '') || 'score';

    // Create PDF with letter size in mm (like OSMD demo does)
    const pdf = new jsPDF({
      orientation: 'portrait',
      unit: 'mm',
      format: 'letter',
    });

    // Add each SVG as a page, scaling to fit letter size
    for (let i = 0; i < svgs.length; i++) {
      if (i > 0) {
        pdf.addPage('letter', 'portrait');
      }
      await pdf.svg(svgs[i], {
        x: 0,
        y: 0,
        width: LETTER_WIDTH_MM,
        height: LETTER_HEIGHT_MM,
      });
    }

    // Get PDF as array buffer and save via adapter
    const pdfData = pdf.output('arraybuffer');
    await files.savePdf(new Uint8Array(pdfData), `${baseName}.pdf`);
  }, [selectedScore, files]);

  // Editor header (always 30% width)
  const editorHeader = (
    <div className='w-[40%] p-3 border-b border-r border-border flex items-center justify-between bg-white'>
      <h2 className='font-semibold text-sm'>
        Editor
        {isCompiling && (
          <span className='ml-2 text-xs text-muted-foreground'>
            compiling...
          </span>
        )}
      </h2>
      <button
        onClick={() => setIsEditorVisible(!isEditorVisible)}
        className='p-1.5 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded transition-colors'
        title={isEditorVisible ? 'Hide editor' : 'Show editor'}
      >
        {isEditorVisible ? <Eye size={16} /> : <EyeOff size={16} />}
      </button>
    </div>
  );

  // Editor body (only when visible)
  const editorBody = (
    <div className='w-[40%] h-full border-r border-border bg-white overflow-hidden'>
      <GenMonacoEditor
        value={genSource}
        onChange={setGenSource}
        error={error}
        placeholder='Select a score or start typing...'
        instrumentGroup={currentInstrumentGroup}
        modPointsForGroup={modPointsForGroup}
        onModPointToggle={handleModPointToggle}
      />
    </div>
  );

  // Editor panel for mobile (combined header + body)
  const editorPanel = (
    <div className='h-full border-r border-border flex flex-col bg-white'>
      <div className='p-3 border-b border-border flex items-center justify-between'>
        <h2 className='font-semibold text-sm'>
          Editor
          {isCompiling && (
            <span className='ml-2 text-xs text-muted-foreground'>
              compiling...
            </span>
          )}
        </h2>
      </div>
      <div className='flex-1 overflow-hidden'>
        <GenMonacoEditor
          value={genSource}
          onChange={setGenSource}
          error={error}
          placeholder='Select a score or start typing...'
          instrumentGroup={currentInstrumentGroup}
          modPointsForGroup={modPointsForGroup}
          onModPointToggle={handleModPointToggle}
        />
      </div>
    </div>
  );

  // Sheet music header (takes remaining width - 70% on desktop)
  const sheetMusicHeader = (
    <div className='flex-1 p-3 border-b border-border flex items-center justify-between bg-white'>
      <h2 className='font-semibold text-sm'>Sheet Music</h2>
      <div className='flex items-center gap-2'>
        {/* Playback controls - only show if playback adapter is available */}
        {playback && playbackData && (
          <div className='flex items-center gap-1.5 mr-2'>
            {!isPlaying ? (
              <button
                onClick={handlePlay}
                disabled={isLoadingPlayback}
                className='flex items-center gap-1 px-2 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed rounded-md transition-colors'
                title='Play'
              >
                <Play size={14} />
              </button>
            ) : (
              <button
                onClick={handlePause}
                className='flex items-center gap-1 px-2 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors'
                title='Pause'
              >
                <Pause size={14} />
              </button>
            )}
            <button
              onClick={handleStop}
              disabled={!isPlaying && currentBeat === 0}
              className='flex items-center gap-1 px-2 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed rounded-md transition-colors'
              title='Stop'
            >
              <Square size={14} />
            </button>
            <input
              type='range'
              min={0}
              max={totalBeats}
              step={0.01}
              value={currentBeat}
              onChange={e => handleSeek(Number(e.target.value))}
              className='w-32 h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer'
              title='Seek'
            />
            <span className='text-xs text-gray-500 min-w-12 text-right'>
              {(() => {
                if (!playbackData) return '0:00';
                const seconds = (currentBeat / (playbackData.tempo / 60));
                const mins = Math.floor(seconds / 60);
                const secs = Math.floor(seconds % 60);
                return `${mins}:${secs.toString().padStart(2, '0')}`;
              })()}
            </span>
          </div>
        )}
        <button
          onClick={exportToPdf}
          className='flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors'
          title='Export to PDF'
        >
          <Download size={14} />
          Export PDF
        </button>
      </div>
    </div>
  );

  // Sheet music body (toolbar + content, for desktop)
  const sheetMusicBody = (
    <div className='h-full flex flex-col bg-white min-w-0'>
      {/* Toolbar */}
      <div className='px-4 py-2 border-b border-border bg-gray-50 flex items-center gap-4 md:gap-6 flex-wrap'>
        {/* Instrument preset dropdown */}
        <div className='flex items-center gap-2'>
          <label className='text-xs font-medium text-gray-600'>
            Instrument:
          </label>
          <select
            value={instrumentIndex}
            onChange={e => {
              const idx = Number(e.target.value);
              setInstrumentIndex(idx);
              if (idx < INSTRUMENT_PRESETS.length) {
                const preset = INSTRUMENT_PRESETS[idx];
                setTransposeIndex(preset.transposeIndex);
                setOctaveShift(preset.octaveShift);
                setClef(preset.clef);
              }
            }}
            className='px-2 py-1 text-xs border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
          >
            {INSTRUMENT_PRESETS.map((preset, index) => (
              <option key={preset.label} value={index}>
                {preset.label}
              </option>
            ))}
            <option value={CUSTOM_PRESET_INDEX}>Custom</option>
          </select>
        </div>

        {/* Octave control - always visible */}
        <div className='flex items-center gap-2'>
          <label className='text-xs font-medium text-gray-600'>Octave:</label>
          <select
            value={octaveShift}
            onChange={e => setOctaveShift(Number(e.target.value))}
            className='px-2 py-1 text-xs border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
          >
            {OCTAVE_OPTIONS.map(option => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        {/* Custom controls - only shown when Custom is selected */}
        {instrumentIndex === CUSTOM_PRESET_INDEX && (
          <>
            <div className='flex items-center gap-2'>
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
            <div className='flex items-center gap-2'>
              <label className='text-xs font-medium text-gray-600'>Clef:</label>
              <select
                value={clef}
                onChange={e => setClef(e.target.value as Clef)}
                className='px-2 py-1 text-xs border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
              >
                {CLEF_OPTIONS.map(option => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </div>
          </>
        )}

        {/* Zoom control - always visible */}
        <div className='flex items-center gap-2'>
          <label className='text-xs font-medium text-gray-600'>Zoom:</label>
          <input
            type='range'
            min='0.3'
            max='1.5'
            step='0.05'
            value={zoom}
            onChange={e => setZoom(Number(e.target.value))}
            className='w-20 md:w-24 h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer'
          />
          <span className='text-xs text-gray-500 w-10'>
            {Math.round(zoom * 100)}%
          </span>
        </div>
      </div>
      <div className='flex-1 overflow-auto p-4'>
        <div ref={sheetMusicRef} className='w-full' />
      </div>
    </div>
  );

  // Sheet music panel content (shared between mobile and desktop)
  const sheetMusicPanel = (
    <div className='h-full flex flex-col bg-white min-w-0'>
      <div className='p-3 border-b border-border flex items-center justify-between'>
        <h2 className='font-semibold text-sm'>Sheet Music</h2>
        <div className='flex items-center gap-2'>
          {/* Playback controls - only show if playback adapter is available */}
          {playback && playbackData && (
            <div className='flex items-center gap-1.5 mr-2'>
              {!isPlaying ? (
                <button
                  onClick={handlePlay}
                  disabled={isLoadingPlayback}
                  className='flex items-center gap-1 px-2 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed rounded-md transition-colors'
                  title='Play'
                >
                  <Play size={14} />
                </button>
              ) : (
                <button
                  onClick={handlePause}
                  className='flex items-center gap-1 px-2 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors'
                  title='Pause'
                >
                  <Pause size={14} />
                </button>
              )}
              <button
                onClick={handleStop}
                disabled={!isPlaying && currentBeat === 0}
                className='flex items-center gap-1 px-2 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed rounded-md transition-colors'
                title='Stop'
              >
                <Square size={14} />
              </button>
              <input
                type='range'
                min={0}
                max={totalBeats}
                step={0.01}
                value={currentBeat}
                onChange={e => handleSeek(Number(e.target.value))}
                className='w-32 h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer'
                title='Seek'
              />
              <span className='text-xs text-gray-500 min-w-12 text-right'>
                {(() => {
                  if (!playbackData) return '0:00';
                  const seconds = (currentBeat / (playbackData.tempo / 60));
                  const mins = Math.floor(seconds / 60);
                  const secs = Math.floor(seconds % 60);
                  return `${mins}:${secs.toString().padStart(2, '0')}`;
                })()}
              </span>
            </div>
          )}
          <button
            onClick={exportToPdf}
            className='flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors'
            title='Export to PDF'
          >
            <Download size={14} />
            Export PDF
          </button>
        </div>
      </div>
      {/* Toolbar */}
      <div className='px-4 py-2 border-b border-border bg-gray-50 flex items-center gap-4 md:gap-6 flex-wrap'>
        {/* Instrument preset dropdown */}
        <div className='flex items-center gap-2'>
          <label className='text-xs font-medium text-gray-600'>
            Instrument:
          </label>
          <select
            value={instrumentIndex}
            onChange={e => {
              const idx = Number(e.target.value);
              setInstrumentIndex(idx);
              if (idx < INSTRUMENT_PRESETS.length) {
                const preset = INSTRUMENT_PRESETS[idx];
                setTransposeIndex(preset.transposeIndex);
                setOctaveShift(preset.octaveShift);
                setClef(preset.clef);
              }
            }}
            className='px-2 py-1 text-xs border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
          >
            {INSTRUMENT_PRESETS.map((preset, index) => (
              <option key={preset.label} value={index}>
                {preset.label}
              </option>
            ))}
            <option value={CUSTOM_PRESET_INDEX}>Custom</option>
          </select>
        </div>

        {/* Octave control - always visible */}
        <div className='flex items-center gap-2'>
          <label className='text-xs font-medium text-gray-600'>Octave:</label>
          <select
            value={octaveShift}
            onChange={e => setOctaveShift(Number(e.target.value))}
            className='px-2 py-1 text-xs border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
          >
            {OCTAVE_OPTIONS.map(option => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        {/* Custom controls - only shown when Custom is selected */}
        {instrumentIndex === CUSTOM_PRESET_INDEX && (
          <>
            <div className='flex items-center gap-2'>
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
            <div className='flex items-center gap-2'>
              <label className='text-xs font-medium text-gray-600'>Clef:</label>
              <select
                value={clef}
                onChange={e => setClef(e.target.value as Clef)}
                className='px-2 py-1 text-xs border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
              >
                {CLEF_OPTIONS.map(option => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </div>
          </>
        )}

        {/* Zoom control - always visible */}
        <div className='flex items-center gap-2'>
          <label className='text-xs font-medium text-gray-600'>Zoom:</label>
          <input
            type='range'
            min='0.3'
            max='1.5'
            step='0.05'
            value={zoom}
            onChange={e => setZoom(Number(e.target.value))}
            className='w-20 md:w-24 h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer'
          />
          <span className='text-xs text-gray-500 w-10'>
            {Math.round(zoom * 100)}%
          </span>
        </div>
      </div>
      <div className='flex-1 overflow-auto p-4'>
        <div ref={sheetMusicRef} className='w-full' />
      </div>
    </div>
  );

  // Mobile/Tablet layout
  if (isMobileOrTablet) {
    return (
      <div className='flex flex-col h-screen w-screen'>
        {/* Side Drawer for score selection */}
        <SideDrawer
          isOpen={isDrawerOpen}
          onClose={() => setIsDrawerOpen(false)}
          title='Scores'
          width={isTablet ? 'wide' : 'default'}
        >
          <Sidebar
            scores={scores}
            onScoreSelect={handleScoreSelect}
            selectedScore={selectedScore}
            onOpenDocs={url ? handleOpenDocs : undefined}
            variant='drawer'
            onClose={() => setIsDrawerOpen(false)}
          />
        </SideDrawer>

        {/* Mobile Header */}
        <MobileHeader
          title={selectedScore?.split('/').pop()?.replace('.gen', '') || 'Gen'}
          onMenuClick={() => setIsDrawerOpen(true)}
          rightContent={
            <button
              onClick={exportToPdf}
              className='p-3 -mr-1 rounded-md hover:bg-gray-100 text-gray-700'
              title='Export to PDF'
            >
              <Download size={20} />
            </button>
          }
        />

        {/* Main content area */}
        <div className='flex-1 flex flex-col min-h-0 overflow-hidden'>
          {/* Show active panel */}
          <div
            className={mobileTab === 'editor' ? 'flex-1 overflow-hidden' : 'hidden'}
            role='tabpanel'
            id='editor-panel'
            aria-labelledby='editor-tab'
          >
            {editorPanel}
          </div>
          <div
            className={mobileTab === 'sheet' ? 'flex-1 overflow-hidden' : 'hidden'}
            role='tabpanel'
            id='sheet-panel'
            aria-labelledby='sheet-tab'
          >
            {sheetMusicPanel}
          </div>
        </div>

        {/* Bottom Tab Bar */}
        <BottomTabBar activeTab={mobileTab} onTabChange={setMobileTab} />
      </div>
    );
  }

  // Desktop layout
  return (
    <div className='flex h-screen w-screen'>
      {/* Sidebar - fixed on desktop */}
      <Sidebar
        scores={scores}
        onScoreSelect={handleScoreSelect}
        selectedScore={selectedScore}
        isCollapsed={isSidebarCollapsed}
        onToggleCollapse={toggleSidebar}
        onOpenDocs={url ? handleOpenDocs : undefined}
        variant='fixed'
      />

      {/* Desktop layout - header row always full width, body changes */}
      <div className='flex-1 h-full flex flex-col'>
        {/* Header row - editor header (40%) + sheet music header (60%) */}
        <div className='flex'>
          {editorHeader}
          {sheetMusicHeader}
        </div>

        {/* Body row - editor body (40% when visible) + sheet music body */}
        <div className='flex-1 flex min-h-0'>
          {isEditorVisible && editorBody}
          <div className={isEditorVisible ? 'w-[60%] h-full' : 'w-full h-full'}>
            {sheetMusicBody}
          </div>
        </div>
      </div>
    </div>
  );
}
