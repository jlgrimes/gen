import { useState } from "react";
import { cn } from "@/lib/utils";
import { File, Music, Folder, ChevronRight } from "lucide-react";

interface ScoreInfo {
  name: string;
  content: string;
}

interface SidebarProps {
  scores: ScoreInfo[];
  onScoreSelect: (score: ScoreInfo) => void;
  selectedScore: string | null;
}

interface FolderNode {
  name: string;
  files: ScoreInfo[];
  folders: Map<string, FolderNode>;
}

function buildFolderTree(scores: ScoreInfo[]): FolderNode {
  const root: FolderNode = { name: "", files: [], folders: new Map() };

  for (const score of scores) {
    const parts = score.name.split("/");
    let current = root;

    for (let i = 0; i < parts.length - 1; i++) {
      const folderName = parts[i];
      if (!current.folders.has(folderName)) {
        current.folders.set(folderName, { name: folderName, files: [], folders: new Map() });
      }
      current = current.folders.get(folderName)!;
    }

    current.files.push(score);
  }

  return root;
}

function FolderItem({
  node,
  depth,
  onScoreSelect,
  selectedScore,
  expandedFolders,
  toggleFolder,
}: {
  node: FolderNode;
  depth: number;
  onScoreSelect: (score: ScoreInfo) => void;
  selectedScore: string | null;
  expandedFolders: Set<string>;
  toggleFolder: (path: string) => void;
}) {
  const isExpanded = expandedFolders.has(node.name);
  const sortedFolders = [...node.folders.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  const sortedFiles = [...node.files].sort((a, b) => a.name.localeCompare(b.name));

  return (
    <li>
      <button
        onClick={() => toggleFolder(node.name)}
        className="w-full flex items-center gap-1 px-2 py-1.5 text-sm rounded-md text-sidebar-foreground hover:bg-sidebar-accent"
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
      >
        <ChevronRight
          className={cn(
            "h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform",
            isExpanded && "rotate-90"
          )}
        />
        <Folder className="h-4 w-4 shrink-0 text-muted-foreground" />
        <span className="truncate">{node.name}</span>
      </button>
      {isExpanded && (
        <ul className="space-y-0.5">
          {sortedFolders.map(([name, folder]) => (
            <FolderItem
              key={name}
              node={folder}
              depth={depth + 1}
              onScoreSelect={onScoreSelect}
              selectedScore={selectedScore}
              expandedFolders={expandedFolders}
              toggleFolder={toggleFolder}
            />
          ))}
          {sortedFiles.map((score) => (
            <li key={score.name}>
              <button
                onClick={() => onScoreSelect(score)}
                className={cn(
                  "w-full flex items-center gap-2 py-1.5 text-sm rounded-md text-sidebar-foreground",
                  selectedScore === score.name
                    ? "bg-sidebar-accent font-medium"
                    : "hover:bg-sidebar-accent"
                )}
                style={{ paddingLeft: `${(depth + 1) * 12 + 20}px` }}
              >
                <File className="h-4 w-4 shrink-0 text-muted-foreground" />
                <span className="truncate">{score.name.split("/").pop()}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}

export function Sidebar({ scores, onScoreSelect, selectedScore }: SidebarProps) {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(() => new Set());
  const tree = buildFolderTree(scores);

  const toggleFolder = (path: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  const sortedFolders = [...tree.folders.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  const sortedRootFiles = [...tree.files].sort((a, b) => a.name.localeCompare(b.name));

  return (
    <div className="w-64 h-full bg-sidebar border-r border-sidebar-border flex flex-col">
      <div className="p-3 border-b border-sidebar-border flex items-center gap-2">
        <Music className="h-4 w-4" />
        <h2 className="font-semibold text-sm text-sidebar-foreground">Scores</h2>
      </div>
      <div className="flex-1 overflow-auto p-2">
        <ul className="space-y-0.5">
          {sortedFolders.map(([name, folder]) => (
            <FolderItem
              key={name}
              node={folder}
              depth={0}
              onScoreSelect={onScoreSelect}
              selectedScore={selectedScore}
              expandedFolders={expandedFolders}
              toggleFolder={toggleFolder}
            />
          ))}
          {sortedRootFiles.map((score) => (
            <li key={score.name}>
              <button
                onClick={() => onScoreSelect(score)}
                className={cn(
                  "w-full flex items-center gap-2 px-3 py-1.5 text-sm rounded-md text-sidebar-foreground",
                  selectedScore === score.name
                    ? "bg-sidebar-accent font-medium"
                    : "hover:bg-sidebar-accent"
                )}
              >
                <File className="h-4 w-4 shrink-0 text-muted-foreground" />
                <span className="truncate">{score.name}</span>
              </button>
            </li>
          ))}
        </ul>
        {scores.length === 0 && (
          <p className="text-sm text-muted-foreground px-3 py-2">
            No scores found
          </p>
        )}
      </div>
    </div>
  );
}

export type { ScoreInfo };
