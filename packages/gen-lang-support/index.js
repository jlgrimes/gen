// Gen Language Support - shared configuration for syntax highlighting
const path = require('path');

module.exports = {
  // Path to TextMate grammar
  grammarPath: path.join(__dirname, 'syntaxes', 'gen.tmLanguage.json'),

  // Path to language configuration
  languageConfigPath: path.join(__dirname, 'language-configuration.json'),

  // Language ID
  languageId: 'gen',

  // File extensions
  extensions: ['.gen'],

  // TextMate scope name
  scopeName: 'source.gen',
};
