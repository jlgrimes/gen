const esbuild = require('esbuild');
const fs = require('fs');
const path = require('path');

const isWatch = process.argv.includes('--watch');

// Copy shared language files from gen-lang-support
function copySharedFiles() {
  const sharedDir = path.join(__dirname, '..', 'gen-lang-support');

  // Copy grammar
  const grammarSrc = path.join(sharedDir, 'syntaxes', 'gen.tmLanguage.json');
  const grammarDest = path.join(__dirname, 'syntaxes', 'gen.tmLanguage.json');
  fs.mkdirSync(path.dirname(grammarDest), { recursive: true });
  fs.copyFileSync(grammarSrc, grammarDest);

  // Copy language configuration
  const configSrc = path.join(sharedDir, 'language-configuration.json');
  const configDest = path.join(__dirname, 'language-configuration.json');
  fs.copyFileSync(configSrc, configDest);

  console.log('Copied shared language files from gen-lang-support');
}

copySharedFiles();

const config = {
  entryPoints: ['src/extension.ts'],
  bundle: true,
  outfile: 'dist/extension.js',
  external: ['vscode'],
  format: 'cjs',
  platform: 'node',
  target: 'node18',
  sourcemap: true,
  minify: !isWatch,
};

if (isWatch) {
  esbuild.context(config).then(ctx => {
    ctx.watch();
    console.log('Watching for changes...');
  });
} else {
  esbuild.build(config).then(() => {
    console.log('Build complete');
  });
}
