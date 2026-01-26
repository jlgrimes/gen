import React from 'react';
import ReactDOM from 'react-dom/client';
import { GenApp } from 'gen-ui';
import { wasmCompiler, browserFiles } from './adapters';
import './app.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <GenApp compiler={wasmCompiler} files={browserFiles} />
  </React.StrictMode>
);
