import React from 'react';
import ReactDOM from 'react-dom/client';
import { GenApp } from 'gen-ui';
import { wasmCompiler, browserFiles } from './adapters';
import '../node_modules/gen-ui/src/index.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <GenApp compiler={wasmCompiler} files={browserFiles} />
  </React.StrictMode>
);
