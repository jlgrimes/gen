import { useState, useRef, useEffect } from "react";
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
  const sheetMusicRef = useRef<HTMLDivElement>(null);
  const osmdRef = useRef<OpenSheetMusicDisplay | null>(null);

  const compileAndRender = async () => {
    try {
      setError(null);
      // Call Rust backend to compile Gen to MusicXML
      const musicXml = await invoke<string>("compile_gen", { source: genSource });

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
    }
  };

  useEffect(() => {
    compileAndRender();
  }, []);

  return (
    <div style={{ display: "flex", height: "100vh" }}>
      {/* Editor Panel */}
      <div style={{ width: "40%", padding: "1rem", borderRight: "1px solid #ccc" }}>
        <h2>Gen Editor</h2>
        <textarea
          value={genSource}
          onChange={(e) => setGenSource(e.target.value)}
          style={{
            width: "100%",
            height: "calc(100% - 100px)",
            fontFamily: "monospace",
            fontSize: "14px",
            padding: "0.5rem",
          }}
        />
        <button
          onClick={compileAndRender}
          style={{
            marginTop: "1rem",
            padding: "0.5rem 1rem",
            fontSize: "1rem",
            cursor: "pointer",
          }}
        >
          Compile & Render
        </button>
        {error && (
          <div style={{ color: "red", marginTop: "1rem", whiteSpace: "pre-wrap" }}>
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
