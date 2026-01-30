import { Code, Music2 } from 'lucide-react';
import { cn } from '@/lib/utils';

type Tab = 'editor' | 'sheet';

interface BottomTabBarProps {
  activeTab: Tab;
  onTabChange: (tab: Tab) => void;
}

export function BottomTabBar({ activeTab, onTabChange }: BottomTabBarProps) {
  return (
    <nav
      role="tablist"
      aria-label="Main navigation"
      className="h-14 border-t border-border bg-white flex pb-safe shrink-0"
    >
      <button
        role="tab"
        aria-selected={activeTab === 'editor'}
        aria-controls="editor-panel"
        onClick={() => onTabChange('editor')}
        className={cn(
          'flex-1 flex flex-col items-center justify-center gap-1 transition-colors',
          activeTab === 'editor'
            ? 'text-primary'
            : 'text-muted-foreground hover:text-foreground'
        )}
      >
        <Code className="h-5 w-5" />
        <span className="text-xs font-medium">Editor</span>
      </button>

      <button
        role="tab"
        aria-selected={activeTab === 'sheet'}
        aria-controls="sheet-panel"
        onClick={() => onTabChange('sheet')}
        className={cn(
          'flex-1 flex flex-col items-center justify-center gap-1 transition-colors',
          activeTab === 'sheet'
            ? 'text-primary'
            : 'text-muted-foreground hover:text-foreground'
        )}
      >
        <Music2 className="h-5 w-5" />
        <span className="text-xs font-medium">Sheet Music</span>
      </button>
    </nav>
  );
}
