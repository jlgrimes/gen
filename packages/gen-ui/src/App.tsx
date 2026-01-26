import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OpenSheetMusicDisplay } from "opensheetmusicdisplay";

function App() {
  const [genSource, setGenSource] = useState(`---
title: Twinkle Twinkle
composer: Traditional
time-signature: 4/4
---
C C G G
A A |oG
F F E E
D D |oC`);
  const [error, setError] = useState<string | null>(null);
  const [isCompiling, setIsCompiling] = useState(false);
  const sheetMusicRef = useRef<HTMLDivElement>(null);
  const osmdRef = useRef<OpenSheetMusicDisplay | null>(null);
  const debounceRef = useRef<NodeJS.Timeout | null>(null);

  const compileAndRender = useCallback(async (source: string) => {
    setIsCompiling(true);
    try {
      setError(null);
      // Call Rust backend to compile Gen to MusicXML (unchecked for real-time)
      const musicXml = await invoke<string>("compile_gen_unchecked", { source });

      // Render MusicXML with OpenSheetMusicDisplay
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
    }, 150); // 150ms debounce

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [genSource, compileAndRender]);

  return (
    <div style={{ display: "flex", height: "100vh" }}>
      {/* Editor Panel */}
      <div style={{ width: "40%", padding: "1rem", borderRight: "1px solid #ccc", display: "flex", flexDirection: "column" }}>
        <h2>Gen Editor {isCompiling && <span style={{ fontSize: "0.8rem", color: "#888" }}>compiling...</span>}</h2>
        <textarea
          value={genSource}
          onChange={(e) => setGenSource(e.target.value)}
          style={{
            flex: 1,
            fontFamily: "monospace",
            fontSize: "14px",
            padding: "0.5rem",
            resize: "none",
          }}
        />
        {error && (
          <div style={{ color: "red", marginTop: "1rem", whiteSpace: "pre-wrap", maxHeight: "100px", overflow: "auto" }}>
            {error}
          </div>
        )}
      </div>

      {/* Sheet Music Panel */}
      <div style={{ flex: 1, padding: "1rem", overflow: "auto" }}>
        <h2>Sheet Music</h2>
        <div ref={sheetMusicRef} style={{ width: "100%", minHeight: "400px" }} />
      </div>
    </div>
  );
}

export default App;
