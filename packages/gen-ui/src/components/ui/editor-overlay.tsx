import { useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import { cn } from '@/lib/utils';

interface EditorOverlayProps {
  isOpen: boolean;
  onClose: () => void;
  children: React.ReactNode;
  title?: string;
  isCompiling?: boolean;
}

export function EditorOverlay({
  isOpen,
  onClose,
  children,
  title = 'Editor',
  isCompiling = false,
}: EditorOverlayProps) {
  const overlayRef = useRef<HTMLDivElement>(null);

  // Handle escape key
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  return (
    <div
      className={cn(
        'fixed inset-0 z-40 transition-all duration-300',
        isOpen ? 'visible' : 'invisible'
      )}
    >
      {/* Semi-transparent backdrop */}
      <div
        className={cn(
          'absolute inset-0 bg-black/30 transition-opacity duration-300',
          isOpen ? 'opacity-100' : 'opacity-0'
        )}
        onClick={onClose}
      />

      {/* Editor panel - slides up from bottom */}
      <div
        ref={overlayRef}
        className={cn(
          'absolute bottom-0 left-0 right-0 bg-white rounded-t-xl shadow-2xl flex flex-col',
          'transform transition-transform duration-300 ease-out',
          'h-[85vh] max-h-[85vh]',
          isOpen ? 'translate-y-0' : 'translate-y-full'
        )}
      >
        {/* Header with drag handle */}
        <div className='flex flex-col items-center pt-2 pb-1 border-b border-border'>
          {/* Drag indicator */}
          <div className='w-10 h-1 bg-gray-300 rounded-full mb-2' />

          <div className='w-full px-4 pb-2 flex items-center justify-between'>
            <h2 className='font-semibold text-sm'>
              {title}
              {isCompiling && (
                <span className='ml-2 text-xs text-muted-foreground'>
                  compiling...
                </span>
              )}
            </h2>
            <button
              onClick={onClose}
              className='p-2 -mr-2 rounded-md hover:bg-gray-100 text-gray-500'
              aria-label='Close editor'
            >
              <X className='h-5 w-5' />
            </button>
          </div>
        </div>

        {/* Editor content */}
        <div className='flex-1 overflow-hidden'>{children}</div>
      </div>
    </div>
  );
}
