import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { OpenSheetMusicDisplay } from "opensheetmusicdisplay";
import { jsPDF } from "jspdf";
import "svg2pdf.js";
import { Download } from "lucide-react";
import { Sidebar, type ScoreInfo } from "@/components/ui/sidebar";
import { GenEditor, type CompileError } from "@/components/GenEditor";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";

// Transposition options using circle of fifths steps
// fifthsSteps: positive = sharps direction, negative = flats direction
const TRANSPOSE_OPTIONS = [
  { label: "C (Concert Pitch)", fifthsSteps: 0 },
  { label: "Bb Instruments", fifthsSteps: -2 },  // C -> F -> Bb (2 steps flat direction)
  { label: "Eb Instruments", fifthsSteps: -3 },  // C -> F -> Bb -> Eb (3 steps flat direction)
  { label: "F Instruments", fifthsSteps: -1 },   // C -> F (1 step flat direction)
] as const;

// Circle of fifths for key signatures (flats direction: F, Bb, Eb, Ab, Db, Gb)
// Starting from C and going counterclockwise (flat direction)
const CIRCLE_OF_FIFTHS = ["C", "G", "D", "A", "E", "B", "F#", "Db", "Ab", "Eb", "Bb", "F"] as const;
// Same circle but with enharmonic equivalents for sharp keys
const CIRCLE_OF_FIFTHS_FLATS = ["C", "G", "D", "A", "E", "Cb", "Gb", "Db", "Ab", "Eb", "Bb", "F"] as const;

// Note names in scale order with their semitone offsets from C
const NOTE_SEMITONES: Record<string, number> = {
  "C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11
};

// Get semitone value of a note (including accidentals)
function getNoteValue(note: string): number {
  const baseName = note.charAt(0).toUpperCase();
  const base = NOTE_SEMITONES[baseName];
  if (base === undefined) return -1;

  let value = base;
  if (note.includes("#")) value += 1;
  if (note.includes("b")) value -= 1;
  return ((value % 12) + 12) % 12;
}

// Convert fifths steps to semitones (each fifth is 7 semitones, or -5 going backwards)
function fifthsToSemitones(fifthsSteps: number): number {
  // Going down by fifths (flat direction): each step is -7 semitones (or +5)
  // We want to transpose notes UP to compensate for instrument transposition
  return ((fifthsSteps * 7) % 12 + 12) % 12;
}

// Transpose a key signature by circle of fifths steps
function transposeKey(key: string, fifthsSteps: number): string {
  if (fifthsSteps === 0) return key;

  // Find current position in circle of fifths
  const isFlat = key.includes("b");
  const circle = isFlat ? CIRCLE_OF_FIFTHS_FLATS : CIRCLE_OF_FIFTHS;

  let currentIndex = circle.findIndex(k => k === key);
  if (currentIndex === -1) {
    // Try the other circle for enharmonic equivalents
    currentIndex = (isFlat ? CIRCLE_OF_FIFTHS : CIRCLE_OF_FIFTHS_FLATS).findIndex(k => k === key);
    if (currentIndex === -1) return key; // Unknown key
  }

  // Move around the circle
  // Negative fifthsSteps means going flat direction (counterclockwise)
  // In our array, going counterclockwise from C means going towards F (index 11), then Bb (10), etc.
  const newIndex = ((currentIndex - fifthsSteps) % 12 + 12) % 12;

  // Choose appropriate enharmonic spelling based on direction
  // If we're going flat direction, prefer flat spellings
  const resultCircle = fifthsSteps < 0 ? CIRCLE_OF_FIFTHS_FLATS : CIRCLE_OF_FIFTHS;
  return resultCircle[newIndex];
}

// Scale degrees for each key (to properly transpose notes within key context)
const SCALE_NOTES = ["C", "D", "E", "F", "G", "A", "B"] as const;

// Transpose a single note by semitones, maintaining musical spelling
function transposeNote(noteToken: string, semitones: number): string {
  if (semitones === 0) return noteToken;

  // Match rhythm prefix, note name with accidentals, dotted, and octave modifiers
  const match = noteToken.match(/^(\/\/\/|\/\/|\/|\|o|o|\|)?([A-Ga-g])([#b])?(\*)?(\^+|_+)?$/);
  if (!match) return noteToken;

  const [, rhythm = "", noteName, accidental = "", dotted = "", octaveMod = ""] = match;

  const upperNote = noteName.toUpperCase();
  const fullNote = upperNote + accidental;
  const currentValue = getNoteValue(fullNote);
  if (currentValue === -1) return noteToken;

  // Calculate new pitch
  const newValue = ((currentValue + semitones) % 12 + 12) % 12;

  // Find the best note name for this pitch
  // Try to use natural notes first, then sharps/flats
  let newNoteName = "";
  for (const natural of SCALE_NOTES) {
    if (NOTE_SEMITONES[natural] === newValue) {
      newNoteName = natural;
      break;
    }
  }

  if (!newNoteName) {
    // Need an accidental - prefer sharp if original had sharp, flat if original had flat
    const preferFlat = accidental === "b";
    for (const natural of SCALE_NOTES) {
      if (NOTE_SEMITONES[natural] === (newValue + 1) % 12) {
        newNoteName = natural + "b";
        break;
      }
      if (NOTE_SEMITONES[natural] === (newValue + 11) % 12) {
        newNoteName = natural + "#";
        break;
      }
    }
    // Prefer flat spelling if original was flat
    if (preferFlat && newNoteName.includes("#")) {
      for (const natural of SCALE_NOTES) {
        if (NOTE_SEMITONES[natural] === (newValue + 1) % 12) {
          newNoteName = natural + "b";
          break;
        }
      }
    }
  }

  // Calculate octave adjustment
  let octaveCount = 0;
  if (octaveMod) {
    octaveCount = octaveMod.includes("^") ? octaveMod.length : -octaveMod.length;
  }

  // Check if we wrapped around octave
  if (currentValue + semitones >= 12) octaveCount += 1;
  if (currentValue + semitones < 0) octaveCount -= 1;

  let newOctaveMod = "";
  if (octaveCount > 0) newOctaveMod = "^".repeat(octaveCount);
  if (octaveCount < 0) newOctaveMod = "_".repeat(-octaveCount);

  return rhythm + newNoteName + dotted + newOctaveMod;
}

// Transpose entire gen source
function transposeGenSource(source: string, fifthsSteps: number): string {
  if (fifthsSteps === 0) return source;

  const semitones = fifthsToSemitones(fifthsSteps);

  const lines = source.split("\n");
  let inMetadata = false;
  const result: string[] = [];

  for (const line of lines) {
    if (line.trim() === "---") {
      inMetadata = !inMetadata;
      result.push(line);
      continue;
    }

    if (inMetadata) {
      // Transpose key-signature in metadata
      const keyMatch = line.match(/^(\s*key-signature:\s*)([A-Ga-g][#b]?)(.*)$/);
      if (keyMatch) {
        const [, prefix, key, suffix] = keyMatch;
        result.push(prefix + transposeKey(key, fifthsSteps) + suffix);
        continue;
      }
      result.push(line);
      continue;
    }

    // Transpose notes in music content
    // Split by whitespace but preserve it
    const tokens = line.split(/(\s+)/);
    const transposedTokens = tokens.map(token => {
      if (/^\s*$/.test(token)) return token; // Preserve whitespace
      if (token === "$" || token.startsWith("/") && token.includes("$")) return token; // Rests
      if (token === "||:" || token === ":||") return token; // Repeats
      return transposeNote(token, semitones);
    });
    result.push(transposedTokens.join(""));
  }

  return result.join("\n");
}

interface CompileResult {
  status: "success" | "error";
  xml?: string;
  error?: CompileError;
}

function App() {
  const [genSource, setGenSource] = useState("");
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
        const scoreList = await invoke<ScoreInfo[]>("list_scores");
        setScores(scoreList);
        // Auto-select first score if available
        if (scoreList.length > 0) {
          setGenSource(scoreList[0].content);
          setSelectedScore(scoreList[0].name);
        }
      } catch (e) {
        console.error("Failed to load scores:", e);
      }
    };
    loadScores();
  }, []);

  const compileAndRender = useCallback(async (source: string, fifthsSteps: number) => {
    if (!source.trim()) return;

    setIsCompiling(true);
    try {
      // Apply transposition before compiling
      const transposedSource = transposeGenSource(source, fifthsSteps);
      const result = await invoke<CompileResult>("compile_gen_unchecked", { source: transposedSource });

      if (result.status === "error" && result.error) {
        setError(result.error);
        return;
      }

      setError(null);

      if (result.xml && sheetMusicRef.current) {
        if (!osmdRef.current) {
          osmdRef.current = new OpenSheetMusicDisplay(sheetMusicRef.current, {
            autoResize: true,
            backend: "svg",
          });
        }
        await osmdRef.current.load(result.xml);
        osmdRef.current.render();
      }
    } catch (e) {
      setError({ message: String(e), line: null, column: null });
    } finally {
      setIsCompiling(false);
    }
  }, []);

  // Debounced compile on source change or transpose change
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      compileAndRender(genSource, TRANSPOSE_OPTIONS[transposeIndex].fifthsSteps);
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
    const svg = sheetMusicRef.current?.querySelector("svg");
    if (!svg) return;

    // Show save dialog
    const filePath = await save({
      defaultPath: `${selectedScore || "score"}.pdf`,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });

    if (!filePath) return; // User cancelled

    const svgWidth = svg.clientWidth || svg.getBoundingClientRect().width;
    const svgHeight = svg.clientHeight || svg.getBoundingClientRect().height;

    const pdf = new jsPDF({
      orientation: svgWidth > svgHeight ? "landscape" : "portrait",
      unit: "pt",
      format: [svgWidth, svgHeight],
    });

    await pdf.svg(svg, { width: svgWidth, height: svgHeight });

    // Get PDF as array buffer and write to file
    const pdfData = pdf.output("arraybuffer");
    await writeFile(filePath, new Uint8Array(pdfData));
  }, [selectedScore]);

  return (
    <div className="flex h-screen w-screen">
      {/* Sidebar - outside of resizable panels */}
      <Sidebar
        scores={scores}
        onScoreSelect={handleScoreSelect}
        selectedScore={selectedScore}
        isCollapsed={isSidebarCollapsed}
        onToggleCollapse={toggleSidebar}
      />

      {/* Main content area with resizable panels */}
      <ResizablePanelGroup orientation="horizontal" className="flex-1 h-full">
        {/* Editor Panel */}
        <ResizablePanel defaultSize={35} minSize={20}>
          <div className="h-full border-r border-border flex flex-col bg-white">
            <div className="p-3 border-b border-border flex items-center justify-between">
              <h2 className="font-semibold text-sm">
                {selectedScore || "Editor"}
                {isCompiling && <span className="ml-2 text-xs text-muted-foreground">compiling...</span>}
              </h2>
            </div>
            <div className="flex-1 overflow-hidden">
              <GenEditor
                value={genSource}
                onChange={setGenSource}
                error={error}
                placeholder="Select a score or start typing..."
              />
            </div>
            {error && (
              <div className="p-3 text-sm text-red-600 border-t border-border bg-red-50 max-h-24 overflow-auto">
                {error.line !== null ? `Line ${error.line}: ` : ""}{error.message}
              </div>
            )}
          </div>
        </ResizablePanel>

        <ResizableHandle />

        {/* Sheet Music Panel */}
        <ResizablePanel defaultSize={65} minSize={30}>
          <div className="h-full flex flex-col bg-white min-w-0">
            <div className="p-3 border-b border-border flex items-center justify-between">
              <h2 className="font-semibold text-sm">Sheet Music</h2>
              <button
                onClick={exportToPdf}
                className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors"
                title="Export to PDF"
              >
                <Download size={14} />
                Export PDF
              </button>
            </div>
            {/* Transpose Toolbar */}
            <div className="px-4 py-2 border-b border-border bg-gray-50 flex items-center gap-3">
              <label className="text-xs font-medium text-gray-600">Transpose:</label>
              <select
                value={transposeIndex}
                onChange={(e) => setTransposeIndex(Number(e.target.value))}
                className="px-2 py-1 text-xs border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                {TRANSPOSE_OPTIONS.map((option, index) => (
                  <option key={option.label} value={index}>
                    {option.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="flex-1 overflow-auto p-4">
              <div ref={sheetMusicRef} className="w-full" />
            </div>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}

export default App;
