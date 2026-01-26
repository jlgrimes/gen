import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OpenSheetMusicDisplay } from "opensheetmusicdisplay";
import { Sidebar, type ScoreInfo } from "@/components/ui/sidebar";

function App() {
  const [genSource, setGenSource] = useState("");
  const [scores, setScores] = useState<ScoreInfo[]>([]);
  const [selectedScore, setSelectedScore] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isCompiling, setIsCompiling] = useState(false);
  const sheetMusicRef = useRef<HTMLDivElement>(null);
  const osmdRef = useRef<OpenSheetMusicDisplay | null>(null);
  const debounceRef = useRef<NodeJS.Timeout | null>(null);

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

  const compileAndRender = useCallback(async (source: string) => {
    if (!source.trim()) return;

    setIsCompiling(true);
    try {
      setError(null);
      const musicXml = await invoke<string>("compile_gen_unchecked", { source });

      if (sheetMusicRef.current) {
        if (!osmdRef.current) {
          osmdRef.current = new OpenSheetMusicDisplay(sheetMusicRef.current, {
            autoResize: true,
            backend: "svg",
          });
        }
        await osmdRef.current.load(musicXml);
        osmdRef.current.render();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsCompiling(false);
    }
  }, []);

  // Debounced compile on source change
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      compileAndRender(genSource);
    }, 150);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [genSource, compileAndRender]);

  const handleScoreSelect = (score: ScoreInfo) => {
    setGenSource(score.content);
    setSelectedScore(score.name);
  };

  return (
    <div className="flex h-screen w-screen">
      {/* Sidebar */}
      <Sidebar
        scores={scores}
        onScoreSelect={handleScoreSelect}
        selectedScore={selectedScore}
      />

      {/* Editor Panel */}
      <div className="w-80 border-r border-border flex flex-col bg-white">
        <div className="p-3 border-b border-border flex items-center justify-between">
          <h2 className="font-semibold text-sm">
            {selectedScore || "Editor"}
            {isCompiling && <span className="ml-2 text-xs text-muted-foreground">compiling...</span>}
          </h2>
        </div>
        <textarea
          value={genSource}
          onChange={(e) => setGenSource(e.target.value)}
          placeholder="Select a score or start typing..."
          className="flex-1 p-3 font-mono text-sm resize-none focus:outline-none"
        />
        {error && (
          <div className="p-3 text-sm text-red-600 border-t border-border bg-red-50 max-h-24 overflow-auto">
            {error}
          </div>
        )}
      </div>

      {/* Sheet Music Panel */}
      <div className="flex-1 flex flex-col bg-white min-w-0">
        <div className="p-3 border-b border-border">
          <h2 className="font-semibold text-sm">Sheet Music</h2>
        </div>
        <div className="flex-1 overflow-auto p-4">
          <div ref={sheetMusicRef} className="w-full" />
        </div>
      </div>
    </div>
  );
}

export default App;
