import { useEffect, useRef, useState } from 'react';

/**
 * Hook that tracks the width of a container element and detects when it changes.
 * Returns the current width and a flag indicating if it changed since last render.
 */
export function useContainerWidth(ref: React.RefObject<HTMLElement | null>) {
  const [width, setWidth] = useState<number | null>(null);
  const previousWidthRef = useRef<number | null>(null);

  useEffect(() => {
    if (!ref.current) return;

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const newWidth = entry.contentRect.width;
        // Only update if width actually changed (ignore height-only changes)
        if (newWidth !== previousWidthRef.current) {
          previousWidthRef.current = newWidth;
          setWidth(newWidth);
        }
      }
    });

    resizeObserver.observe(ref.current);

    // Set initial width
    const initialWidth = ref.current.offsetWidth;
    previousWidthRef.current = initialWidth;
    setWidth(initialWidth);

    return () => {
      resizeObserver.disconnect();
    };
  }, [ref]);

  return width;
}
