import { describe, expect, it } from 'vitest';
import { auditLabel, describeUserAgent, suggestedPasskeyName } from '../src/lib/account';

const mac = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15';
const chromeWindows = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36';
const edge = `${chromeWindows} Edg/130.0`;
const iphone = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1';

describe('account display helpers', () => {
  it('names devices people recognise', () => {
    expect(describeUserAgent(mac)).toBe('Safari on macOS');
    expect(describeUserAgent(chromeWindows)).toBe('Chrome on Windows');
    expect(describeUserAgent(edge)).toBe('Edge on Windows');
    expect(describeUserAgent(iphone)).toBe('Safari on iOS');
    expect(describeUserAgent(null)).toBe('Unknown device');
  });

  it('suggests a passkey name from the platform', () => {
    expect(suggestedPasskeyName(mac)).toBe('macOS passkey');
    expect(suggestedPasskeyName('curl/8.0')).toBe('Passkey');
  });

  it('labels audit actions in plain language', () => {
    expect(auditLabel('user.revoke_sessions')).toBe('Signed out everywhere');
    expect(auditLabel('something.else')).toBe('something.else');
  });
});
