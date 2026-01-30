import React from 'react';
import ReactDOM from 'react-dom/client';
import { GenApp } from 'gen-ui';
import { wasmCompiler, browserFiles, wasmPlayback } from './adapters';
import { scores } from 'gen-scores';
import './app.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <GenApp compiler={wasmCompiler} files={browserFiles} playback={wasmPlayback} scores={scores} />
  </React.StrictMode>
);
