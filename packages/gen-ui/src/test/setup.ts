import { expect, afterEach, beforeAll } from 'vitest';
import { cleanup } from '@testing-library/react';
import * as matchers from '@testing-library/jest-dom/matchers';
import { Canvas } from 'canvas';

expect.extend(matchers);

// Mock canvas for OSMD
beforeAll(() => {
  global.HTMLCanvasElement.prototype.getContext = function() {
    return new Canvas(200, 200).getContext('2d');
  } as any;
});

afterEach(() => {
  cleanup();
});
