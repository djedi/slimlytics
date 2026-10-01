import { describe, expect, it } from 'vitest';
import { niceTicks, trendPaths } from '../src/lib/ui';

describe('traffic chart helpers', () => {
  it('rounds the y-axis to readable ticks that cover the data', () => {
    expect(niceTicks(0)).toEqual([0, 1]);
    expect(niceTicks(7)).toEqual([0, 2, 4, 6, 8]);
    expect(niceTicks(65)).toEqual([0, 20, 40, 60, 80]);
    expect(niceTicks(1234)).toEqual([0, 500, 1000, 1500]);
    expect(niceTicks(3)).toEqual([0, 1, 2, 3]);
  });

  it('can draw several series against one shared maximum', () => {
    // With a shared max of 20, a series peaking at 10 reaches only half the height.
    const { line } = trendPaths([0, 10], 100, 40, 20);
    expect(line.endsWith(' 100,20')).toBe(true);
  });
});
