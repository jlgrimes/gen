import { Menu } from 'lucide-react';

interface MobileHeaderProps {
  title: string;
  onMenuClick: () => void;
  rightContent?: React.ReactNode;
}

export function MobileHeader({ title, onMenuClick, rightContent }: MobileHeaderProps) {
  return (
    <header className="h-14 px-2 border-b border-border bg-white flex items-center justify-between shrink-0">
      {/* Left: Hamburger menu */}
      <button
        onClick={onMenuClick}
        className="p-3 -ml-1 rounded-md hover:bg-gray-100 text-gray-700"
        aria-label="Open menu"
      >
        <Menu className="h-5 w-5" />
      </button>

      {/* Center: Title */}
      <h1 className="font-semibold text-sm text-gray-900 absolute left-1/2 -translate-x-1/2">
        {title}
      </h1>

      {/* Right: Actions */}
      <div className="flex items-center gap-1">{rightContent}</div>
    </header>
  );
}
