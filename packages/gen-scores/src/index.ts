export interface ScoreInfo {
  name: string;
  content: string;
}

// Import all .gen files using Vite's glob import with ?raw
const scoreModules = import.meta.glob('../scores/**/*.gen', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>;

function parseScores(): ScoreInfo[] {
  const scores: ScoreInfo[] = [];

  for (const [path, content] of Object.entries(scoreModules)) {
    // Extract category and filename from path like "../scores/classics/twinkle.gen"
    const match = path.match(/\.\.\/scores\/(.+)\.gen$/);
    if (!match) continue;

    scores.push({
      name: `${match[1]}.gen`,
      content: content,
    });
  }

  return scores;
}

export const scores = parseScores();
