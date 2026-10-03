// Small display helpers for the account security and admin pages.

/** "Chrome on macOS" from a user-agent string; good enough to recognise your own devices. */
export function describeUserAgent(userAgent?: string | null): string {
  if (!userAgent) return 'Unknown device';
  const ua = userAgent;
  const browser = /Edg\//.test(ua)
    ? 'Edge'
    : /OPR\/|Opera/.test(ua)
      ? 'Opera'
      : /Firefox\//.test(ua)
        ? 'Firefox'
        : /Chrome\/|CriOS\//.test(ua)
          ? 'Chrome'
          : /Safari\//.test(ua)
            ? 'Safari'
            : /slimlytics/i.test(ua)
              ? 'Slimlytics CLI'
              : null;
  const system = /iPhone|iPad|iPod/.test(ua)
    ? 'iOS'
    : /Android/.test(ua)
      ? 'Android'
      : /Mac OS X|Macintosh/.test(ua)
        ? 'macOS'
        : /Windows/.test(ua)
          ? 'Windows'
          : /CrOS/.test(ua)
            ? 'ChromeOS'
            : /Linux/.test(ua)
              ? 'Linux'
              : null;
  if (browser && system) return `${browser} on ${system}`;
  return browser ?? system ?? ua.slice(0, 60);
}

/** A suggested name for a new passkey, based on where it is being created. */
export function suggestedPasskeyName(userAgent: string): string {
  const system = describeUserAgent(userAgent).split(' on ').pop() ?? '';
  return ['macOS', 'iOS', 'Android', 'Windows', 'ChromeOS', 'Linux'].includes(system) ? `${system} passkey` : 'Passkey';
}

export function formatDate(value?: string | null): string {
  if (!value) return 'Never';
  return new Date(value).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

export function formatDateTime(value?: string | null): string {
  if (!value) return 'Never';
  return new Date(value).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' });
}

const auditLabels: Record<string, string> = {
  'user.disable': 'Disabled account',
  'user.enable': 'Re-enabled account',
  'user.revoke_sessions': 'Signed out everywhere',
  'user.delete': 'Deleted account',
  'admin.grant': 'Granted admin access',
  'admin.revoke': 'Removed admin access',
  'passkeys.reset': 'Reset passkeys'
};

export function auditLabel(action: string): string {
  return auditLabels[action] ?? action;
}
