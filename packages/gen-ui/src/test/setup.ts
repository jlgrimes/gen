import { expect, afterEach, beforeAll } from 'vitest';
import { cleanup } from '@testing-library/react';
import * as matchers from '@testing-library/jest-dom/matchers';

expect.extend(matchers);

// Mock canvas for OSMD - try to use canvas package if available, otherwise use basic mock
beforeAll(async () => {
  try {
    const { Canvas } = await import('canvas');
    global.HTMLCanvasElement.prototype.getContext = function() {
      return new Canvas(200, 200).getContext('2d');
    } as any;
  } catch {
    // canvas not available, use basic mock
    global.HTMLCanvasElement.prototype.getContext = function() {
      return {} as any;
    };
  }
});

afterEach(() => {
  cleanup();
});
