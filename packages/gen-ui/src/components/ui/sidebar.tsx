import { cn } from "@/lib/utils";
import { File, Music } from "lucide-react";

interface ScoreInfo {
  name: string;
  content: string;
}

interface SidebarProps {
  scores: ScoreInfo[];
  onScoreSelect: (score: ScoreInfo) => void;
  selectedScore: string | null;
}

export function Sidebar({ scores, onScoreSelect, selectedScore }: SidebarProps) {
  return (
    <div className="w-64 h-full bg-sidebar border-r border-sidebar-border flex flex-col">
      <div className="p-3 border-b border-sidebar-border flex items-center gap-2">
        <Music className="h-4 w-4" />
        <h2 className="font-semibold text-sm text-sidebar-foreground">Scores</h2>
      </div>
      <div className="flex-1 overflow-auto p-2">
        <ul className="space-y-0.5">
          {scores.map((score) => (
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
