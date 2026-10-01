import { describe, expect, it } from 'vitest';
import { activeVisitorCount, countryLabel, flagEmoji, minuteBuckets, relativeTime } from '../src/lib/ui';

const now = new Date('2026-10-01T12:00:00Z').getTime();
const at = (secondsAgo: number) => new Date(now - secondsAgo * 1000).toISOString();

describe('spy helpers', () => {
  it('turns country codes into flags and names, with an honest unknown', () => {
    expect(flagEmoji('US')).toBe('🇺🇸');
    expect(flagEmoji('de')).toBe('🇩🇪');
    expect(flagEmoji(undefined)).toBe('🌐');
    expect(flagEmoji('Unknown')).toBe('🌐');
    expect(countryLabel('US')).toBe('United States');
    expect(countryLabel(null)).toBe('Unknown location');
  });

  it('describes recent times relatively', () => {
    expect(relativeTime(at(5), now)).toBe('just now');
    expect(relativeTime(at(125), now)).toBe('2m ago');
    expect(relativeTime(at(3 * 3600 + 5), now)).toBe('3h ago');
    expect(relativeTime(at(3 * 86400), now)).toBe('3d ago');
  });

  it('buckets events per minute, newest last', () => {
    const buckets = minuteBuckets([{ timestamp: at(10) }, { timestamp: at(30) }, { timestamp: at(70) }, { timestamp: at(4000) }], now, 3);
    expect(buckets).toEqual([0, 1, 2]);
  });

  it('counts distinct recently active visitors', () => {
    const events = [
      { visitorId: 'a', timestamp: at(30) },
      { visitorId: 'a', timestamp: at(60) },
      { visitorId: 'b', timestamp: at(200) },
      { visitorId: 'c', timestamp: at(400) }
    ];
    expect(activeVisitorCount(events, now, 5)).toBe(2);
  });
});
