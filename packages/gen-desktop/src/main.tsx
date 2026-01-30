import React from 'react';
import ReactDOM from 'react-dom/client';
import { GenApp } from 'gen-ui';
import { tauriCompiler, tauriFiles, tauriPlayback, tauriUrl } from './adapters';
import { scores } from 'gen-scores';
import './app.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <GenApp compiler={tauriCompiler} files={tauriFiles} playback={tauriPlayback} url={tauriUrl} scores={scores} />
  </React.StrictMode>
);
