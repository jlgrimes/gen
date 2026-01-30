import { useState, useEffect } from "react";
import { cn } from "@/lib/utils";
import { File, Music, Folder, ChevronRight, PanelLeftClose, PanelLeft, BookOpen } from "lucide-react";
import type { ScoreInfo } from "../../types";

interface SidebarProps {
  scores: ScoreInfo[];
  onScoreSelect: (score: ScoreInfo) => void;
  selectedScore: string | null;
  isCollapsed?: boolean;
  onToggleCollapse?: () => void;
  onOpenDocs?: () => void;
  variant?: 'fixed' | 'drawer';
  onClose?: () => void;
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

export function Sidebar({ scores, onScoreSelect, selectedScore, isCollapsed, onToggleCollapse, onOpenDocs, variant = 'fixed', onClose }: SidebarProps) {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(() => new Set());
  const tree = buildFolderTree(scores);

  // Auto-expand folders containing the selected score
  useEffect(() => {
    if (selectedScore && selectedScore.includes("/")) {
      const parts = selectedScore.split("/");
      const foldersToExpand = parts.slice(0, -1); // All parts except the filename
      setExpandedFolders((prev) => {
        const next = new Set(prev);
        for (const folder of foldersToExpand) {
          next.add(folder);
        }
        return next;
      });
    }
  }, [selectedScore]);

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

  // Drawer variant: no collapse, used inside SideDrawer
  if (variant === 'drawer') {
    const handleScoreSelectAndClose = (score: ScoreInfo) => {
      onScoreSelect(score);
      onClose?.();
    };

    return (
      <div className="h-full flex flex-col">
        <div className="flex-1 overflow-auto p-2">
          <ul className="space-y-0.5">
            {sortedFolders.map(([name, folder]) => (
              <FolderItem
                key={name}
                node={folder}
                depth={0}
                onScoreSelect={handleScoreSelectAndClose}
                selectedScore={selectedScore}
                expandedFolders={expandedFolders}
                toggleFolder={toggleFolder}
              />
            ))}
            {sortedRootFiles.map((score) => (
              <li key={score.name}>
                <button
                  onClick={() => handleScoreSelectAndClose(score)}
                  className={cn(
                    "w-full flex items-center gap-2 px-3 py-2.5 text-sm rounded-md text-sidebar-foreground",
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
        {/* Footer with docs link */}
        <div className="p-2 border-t border-sidebar-border">
          {onOpenDocs ? (
            <button
              onClick={onOpenDocs}
              className="w-full flex items-center gap-2 px-3 py-2.5 text-sm rounded-md text-muted-foreground hover:text-sidebar-foreground hover:bg-sidebar-accent transition-colors"
            >
              <BookOpen className="h-4 w-4 shrink-0" />
              <span>Documentation</span>
            </button>
          ) : (
            <a
              href="https://docs.gen.band"
              target="_blank"
              rel="noopener noreferrer"
              className="w-full flex items-center gap-2 px-3 py-2.5 text-sm rounded-md text-muted-foreground hover:text-sidebar-foreground hover:bg-sidebar-accent transition-colors"
            >
              <BookOpen className="h-4 w-4 shrink-0" />
              <span>Documentation</span>
            </a>
          )}
        </div>
      </div>
    );
  }

  // Fixed variant: collapsed state
  if (isCollapsed) {
    return (
      <div className="h-full bg-sidebar border-r border-sidebar-border flex flex-col items-center py-3 px-1">
        <button
          onClick={onToggleCollapse}
          className="p-2 rounded-md hover:bg-sidebar-accent text-sidebar-foreground"
          title="Expand sidebar"
        >
          <PanelLeft className="h-4 w-4" />
        </button>
      </div>
    );
  }

  // Fixed variant: expanded state
  return (
    <div className="w-64 h-full bg-sidebar border-r border-sidebar-border flex flex-col">
      <div className="p-3 border-b border-sidebar-border flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Music className="h-4 w-4" />
          <h2 className="font-semibold text-sm text-sidebar-foreground">Scores</h2>
        </div>
        <button
          onClick={onToggleCollapse}
          className="p-1 rounded-md hover:bg-sidebar-accent text-sidebar-foreground"
          title="Collapse sidebar"
        >
          <PanelLeftClose className="h-4 w-4" />
        </button>
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
      {/* Footer with docs link */}
      <div className="p-2 border-t border-sidebar-border">
        {onOpenDocs ? (
          <button
            onClick={onOpenDocs}
            className="w-full flex items-center gap-2 px-3 py-1.5 text-sm rounded-md text-muted-foreground hover:text-sidebar-foreground hover:bg-sidebar-accent transition-colors"
          >
            <BookOpen className="h-4 w-4 shrink-0" />
            <span>Documentation</span>
          </button>
        ) : (
          <a
            href="https://docs.gen.band"
            target="_blank"
            rel="noopener noreferrer"
            className="w-full flex items-center gap-2 px-3 py-1.5 text-sm rounded-md text-muted-foreground hover:text-sidebar-foreground hover:bg-sidebar-accent transition-colors"
          >
            <BookOpen className="h-4 w-4 shrink-0" />
            <span>Documentation</span>
          </a>
        )}
      </div>
    </div>
  );
}

