import * as vscode from 'vscode';
import { exec } from 'child_process';
import { promisify } from 'util';
import * as path from 'path';
import * as fs from 'fs';

const execAsync = promisify(exec);

interface Diagnostic {
  message: string;
  line: number;
  column: number;
  end_line: number;
  end_column: number;
  severity: string;
}

let diagnosticCollection: vscode.DiagnosticCollection;
let genCompilerPath: string | null = null;
let pendingLints: Map<string, NodeJS.Timeout> = new Map();

function isGenDocument(doc: vscode.TextDocument): boolean {
  return doc.languageId === 'gen' || doc.fileName.endsWith('.gen');
}

export function activate(context: vscode.ExtensionContext) {
  console.log('Gen Language extension activated');

  diagnosticCollection = vscode.languages.createDiagnosticCollection('gen');
  context.subscriptions.push(diagnosticCollection);

  // Find gen compiler
  findGenCompiler(context).then(compilerPath => {
    genCompilerPath = compilerPath;
    if (compilerPath) {
      console.log(`Found gen compiler at: ${compilerPath}`);
    } else {
      console.log('Gen compiler not found, using inline validation only');
    }

    // Lint all currently open gen documents after compiler is found
    vscode.workspace.textDocuments.forEach(doc => {
      if (isGenDocument(doc)) {
        lintDocument(doc);
      }
    });
  });

  // Lint on document open
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(doc => {
      if (isGenDocument(doc)) {
        lintDocument(doc);
      }
    })
  );

  // Lint on document change (debounced)
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument(event => {
      if (isGenDocument(event.document)) {
        const uri = event.document.uri.toString();

        // Cancel any pending lint for this document
        const existing = pendingLints.get(uri);
        if (existing) {
          clearTimeout(existing);
        }

        // Schedule new lint with longer debounce
        const timeout = setTimeout(() => {
          pendingLints.delete(uri);
          lintDocument(event.document);
        }, 500);

        pendingLints.set(uri, timeout);
      }
    })
  );

  // Clear diagnostics when document closes
  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument(doc => {
      const uri = doc.uri.toString();
      const pending = pendingLints.get(uri);
      if (pending) {
        clearTimeout(pending);
        pendingLints.delete(uri);
      }
      diagnosticCollection.delete(doc.uri);
    })
  );
}

async function findGenCompiler(context: vscode.ExtensionContext): Promise<string | null> {
  // Check common locations - prefer debug build during development (gets updated more often)
  const possiblePaths = [
    // Workspace-relative paths (for development) - debug first
    path.join(vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '', 'target/debug/gen'),
    path.join(vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '', 'target/release/gen'),
    // Global paths
    '/usr/local/bin/gen',
    '/opt/homebrew/bin/gen',
    // User cargo bin
    path.join(process.env.HOME || '', '.cargo/bin/gen'),
  ];

  for (const p of possiblePaths) {
    if (fs.existsSync(p)) {
      return p;
    }
  }

  // Try to find via 'which' command
  try {
    const { stdout } = await execAsync('which gen');
    const trimmed = stdout.trim();
    if (trimmed && fs.existsSync(trimmed)) {
      return trimmed;
    }
  } catch {
    // gen not found in PATH
  }

  return null;
}

async function lintDocument(document: vscode.TextDocument) {
  const text = document.getText();
  const diagnostics: vscode.Diagnostic[] = [];

  // If we have the gen compiler, use it for full validation
  if (genCompilerPath) {
    try {
      const result = await lintWithCompiler(text, genCompilerPath);
      diagnostics.push(...result);
    } catch (error) {
      // Fall back to inline validation
      diagnostics.push(...lintInline(text));
    }
  } else {
    // Use inline validation only
    diagnostics.push(...lintInline(text));
  }

  diagnosticCollection.set(document.uri, diagnostics);
}

function createDiagnostic(range: vscode.Range, message: string, severity: vscode.DiagnosticSeverity): vscode.Diagnostic {
  const diagnostic = new vscode.Diagnostic(range, message, severity);
  diagnostic.source = 'gen';
  return diagnostic;
}

async function lintWithCompiler(source: string, compilerPath: string): Promise<vscode.Diagnostic[]> {
  const diagnostics: vscode.Diagnostic[] = [];
  const lines = source.split('\n');

  try {
    // Write source to temp file
    const tmpFile = path.join(require('os').tmpdir(), `gen-lint-${Date.now()}.gen`);
    fs.writeFileSync(tmpFile, source);

    try {
      // Run gen compiler to validate
      await execAsync(`"${compilerPath}" "${tmpFile}"`, { timeout: 5000 });
      // No errors if command succeeds
    } catch (error: any) {
      // Parse error output
      const stderr = error.stderr || error.message || '';
      const stdout = error.stdout || '';
      const output = stderr + stdout;

      // Parse "Parse error at line X, column Y: message"
      const parseMatch = output.match(/Parse error at line (\d+), column (\d+): (.+)/);
      if (parseMatch) {
        const line = parseInt(parseMatch[1], 10) - 1;
        const column = parseInt(parseMatch[2], 10) - 1;
        const message = parseMatch[3].trim();
        const lineLen = lines[line]?.length || 1;
        const range = new vscode.Range(line, column, line, lineLen);
        diagnostics.push(createDiagnostic(range, message, vscode.DiagnosticSeverity.Error));
      }

      // Parse "Semantic error at measure X: message"
      const semanticMatch = output.match(/Semantic error at measure (\d+): (.+)/);
      if (semanticMatch) {
        const measureNum = parseInt(semanticMatch[1], 10);
        const message = semanticMatch[2].trim();
        // Find line for this measure
        const line = findMeasureLine(source, measureNum);
        const lineLen = lines[line]?.length || 1;
        const range = new vscode.Range(line, 0, line, lineLen);
        diagnostics.push(createDiagnostic(range, message, vscode.DiagnosticSeverity.Error));
      }

      // Parse "Invalid metadata: message"
      const metadataMatch = output.match(/Invalid metadata: (.+)/);
      if (metadataMatch) {
        const message = metadataMatch[1].trim();
        // Find the metadata block and highlight it
        const metaEndLine = lines.findIndex((l, i) => i > 0 && l.trim() === '---');
        const range = new vscode.Range(0, 0, metaEndLine > 0 ? metaEndLine : 0, lines[metaEndLine > 0 ? metaEndLine : 0]?.length || 3);
        diagnostics.push(createDiagnostic(range, message, vscode.DiagnosticSeverity.Error));
      }
    } finally {
      // Clean up temp file
      try { fs.unlinkSync(tmpFile); } catch {}
    }
  } catch (error) {
    console.error('Lint error:', error);
  }

  return diagnostics;
}

function findMeasureLine(source: string, measureNum: number): number {
  const lines = source.split('\n');
  let measureIndex = 0;
  let inMetadata = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    // Track YAML metadata
    if (line === '---') {
      inMetadata = !inMetadata;
      continue;
    }
    if (inMetadata) continue;

    // Skip empty lines and comments
    if (!line || line.startsWith('//')) continue;

    measureIndex++;
    if (measureIndex === measureNum) {
      return i;
    }
  }

  return 0;
}

function lintInline(source: string): vscode.Diagnostic[] {
  const diagnostics: vscode.Diagnostic[] = [];
  const lines = source.split('\n');
  let inMetadata = false;
  let metadataStart = -1;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // Track YAML metadata
    if (trimmed === '---') {
      if (!inMetadata) {
        inMetadata = true;
        metadataStart = i;
      } else {
        inMetadata = false;
      }
      continue;
    }

    if (inMetadata) {
      // Basic metadata validation
      if (trimmed && !trimmed.includes(':') && !trimmed.startsWith('#')) {
        const range = new vscode.Range(i, 0, i, line.length);
        diagnostics.push(createDiagnostic(
          range,
          'Invalid metadata format. Expected "key: value"',
          vscode.DiagnosticSeverity.Warning
        ));
      }
      continue;
    }

    // Skip empty lines and comments
    if (!trimmed || trimmed.startsWith('//')) continue;

    // Check for unmatched brackets
    const openBrackets = (line.match(/\[/g) || []).length;
    const closeBrackets = (line.match(/\]/g) || []).length;
    if (openBrackets !== closeBrackets) {
      const range = new vscode.Range(i, 0, i, line.length);
      diagnostics.push(createDiagnostic(
        range,
        'Unmatched brackets',
        vscode.DiagnosticSeverity.Error
      ));
    }

    // Check for invalid note names
    const invalidNoteMatch = line.match(/(?<![A-Za-z_^])([HIJKLMNOPQRSTUVWXYZ])(?![a-z])/);
    if (invalidNoteMatch) {
      const col = line.indexOf(invalidNoteMatch[1]);
      const range = new vscode.Range(i, col, i, col + 1);
      diagnostics.push(createDiagnostic(
        range,
        `Invalid note name '${invalidNoteMatch[1]}'. Valid notes are A-G`,
        vscode.DiagnosticSeverity.Error
      ));
    }
  }

  // Check if metadata block was never closed
  if (inMetadata) {
    const range = new vscode.Range(metadataStart, 0, metadataStart, 3);
    diagnostics.push(createDiagnostic(
      range,
      'Unclosed metadata block. Missing closing "---"',
      vscode.DiagnosticSeverity.Error
    ));
  }

  return diagnostics;
}

export function deactivate() {
  diagnosticCollection.dispose();
}
