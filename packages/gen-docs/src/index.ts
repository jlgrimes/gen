/**
 * Gen Documentation Package
 *
 * Provides access to Gen language documentation, examples, and guides.
 */

export interface DocMetadata {
  title: string;
  path: string;
  version: string;
}

/**
 * Available documentation files
 */
export const docs = {
  v1: {
    basics: '/docs/v1/gen basics.md',
    examples: '/docs/v1/examples.md',
    compiler: '/docs/v1/compiler.md',
    ui: '/docs/v1/gen-ui.md',
  },
  welcome: '/docs/Welcome.md',
} as const;

/**
 * Get documentation file path
 */
export function getDocPath(doc: keyof typeof docs.v1 | 'welcome'): string {
  if (doc === 'welcome') {
    return docs.welcome;
  }
  return docs.v1[doc];
}

/**
 * List all available documentation
 */
export function listDocs(): DocMetadata[] {
  return [
    { title: 'Welcome', path: docs.welcome, version: 'v1' },
    { title: 'Gen Basics', path: docs.v1.basics, version: 'v1' },
    { title: 'Examples', path: docs.v1.examples, version: 'v1' },
    { title: 'Compiler', path: docs.v1.compiler, version: 'v1' },
    { title: 'UI', path: docs.v1.ui, version: 'v1' },
  ];
}

export default {
  docs,
  getDocPath,
  listDocs,
};
