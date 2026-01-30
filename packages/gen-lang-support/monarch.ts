/**
 * Monarch tokenizer for Gen music notation language.
 *
 * This is the Monaco editor equivalent of the TextMate grammar.
 * Keep in sync with:
 * - SYNTAX_RULES.md (source of truth)
 * - syntaxes/gen.tmLanguage.json (VS Code)
 */

import type * as monaco from 'monaco-editor';

export const genLanguageConfig: monaco.languages.LanguageConfiguration = {
  comments: {
    lineComment: '//',
  },
  brackets: [
    ['[', ']'],
    ['(', ')'],
  ],
  autoClosingPairs: [
    { open: '[', close: ']' },
    { open: '(', close: ')' },
    { open: '"', close: '"' },
  ],
  surroundingPairs: [
    { open: '[', close: ']' },
    { open: '(', close: ')' },
    { open: '"', close: '"' },
  ],
};

export const genMonarchTokens: monaco.languages.IMonarchLanguage = {
  tokenizer: {
    root: [
      // YAML frontmatter
      [/^---$/, { token: 'delimiter', next: '@frontmatter' }],

      // Comments
      [/\/\/.*$/, 'comment'],

      // Annotations (must come before notes to match @ first)
      // Note: Use [@] to match literal @ since @ is special in Monarch (references states)
      // Instrument group octave: @Eb:^, @Bb:_, etc.
      [/[@](Eb|Bb|F|C|G):(\^+|_+)/, 'annotation'],
      // Key change: @key:G, @key:Bbm, @key:##
      [/[@]key:([A-G][#b]?m?|#{1,4}|b{1,4})/, 'annotation'],
      // Chord: @ch:Cmaj7, @ch:Dm/, etc.
      [/[@]ch:[A-G][#b]?[^\s]*/, 'annotation'],
      // Measure octave: @:^, @:__
      [/[@]:(\^+|_+)/, 'annotation'],
      // Pickup measure
      [/[@]pickup/, 'annotation'],

      // Repeat markers
      [/\|\|:/, 'keyword'],
      [/:\|\|/, 'keyword'],

      // Endings (1. or 2.)
      [/[12]\./, 'keyword'],

      // Brackets with optional octave prefix
      // Opening: ^[ or _[ or just [
      [/(\^+|_+)?\[/, 'delimiter.bracket'],
      // Closing: ]3/ or ]5 or just ]
      [/\]([0-9]+)?(\/+|p|o)?(\*)?/, 'delimiter.bracket'],

      // Rests: $ with optional rhythm
      [/\$(\/+|p|o)?(\*)?/, 'variable'],

      // Notes: ^C#/ or _Db* or just E
      // Match octave prefix, note name, optional accidental, optional rhythm, optional dot
      [/(\^+|_+)?[A-G][#b%]?(\/+|p|o)?(\*)?/, {
        cases: {
          '@default': 'variable',
        },
      }],

      // Ties
      [/-/, 'operator'],

      // Slurs
      [/[()]/, 'delimiter.parenthesis'],

      // Whitespace
      [/\s+/, 'white'],
    ],

    frontmatter: [
      // End of frontmatter
      [/^---$/, { token: 'delimiter', next: '@pop' }],
      // Key-value pairs
      [/^(\w[\w-]*)(:)(.*)$/, ['type', 'delimiter', 'string']],
      // Anything else in frontmatter
      [/./, 'string'],
    ],
  },
};

/**
 * Register the Gen language with Monaco editor.
 * Call this once before creating any editors.
 */
export function registerGenLanguage(monacoInstance: typeof monaco): void {
  // Register language ID
  monacoInstance.languages.register({
    id: 'gen',
    extensions: ['.gen'],
    aliases: ['Gen', 'gen'],
  });

  // Set language configuration
  monacoInstance.languages.setLanguageConfiguration('gen', genLanguageConfig);

  // Set tokenizer
  monacoInstance.languages.setMonarchTokensProvider('gen', genMonarchTokens);
}
