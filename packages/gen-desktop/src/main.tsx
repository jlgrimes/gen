import React from 'react';
import ReactDOM from 'react-dom/client';
import { GenApp } from 'gen-ui';
import { tauriCompiler, tauriFiles } from './adapters';
import { scores } from 'gen-scores';
import './app.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <GenApp compiler={tauriCompiler} files={tauriFiles} scores={scores} />
  </React.StrictMode>
);
