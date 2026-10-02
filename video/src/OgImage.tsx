import { AbsoluteFill } from 'remotion';
import { Backdrop, LogoMark } from './components';
import { body, colors, display } from './theme';

// 1200x630 Open Graph / Twitter card. Platforms often show it ~500 px wide, so type is large
// and the preview is decorative rather than something to read.
const trend = [18, 24, 21, 30, 27, 36, 33, 42, 39, 50, 46, 58, 54, 66];

function chartPath(width: number, height: number) {
  const max = Math.max(...trend);
  const points = trend.map((value, index) => [(index / (trend.length - 1)) * width, height - (value / max) * height * 0.9]);
  return points.reduce((path, [x, y], index) => {
    if (!index) return `M${x},${y}`;
    const [px, py] = points[index - 1];
    const mid = (px + x) / 2;
    return `${path} C${mid},${py} ${mid},${y} ${x},${y}`;
  }, '');
}

export function OgImage() {
  const line = chartPath(380, 120);
  const feed = [
    { flag: '🇺🇸', page: '/pricing', place: 'Austin' },
    { flag: '🇩🇪', page: '/docs/mcp', place: 'Berlin' },
    { flag: '🇯🇵', page: 'signup 🎯', place: 'Tokyo' }
  ];
  return (
    <AbsoluteFill>
      <Backdrop />
      <AbsoluteFill style={{ padding: '64px 72px', fontFamily: body, color: colors.text }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 18 }}>
          <LogoMark size={64} />
          <span style={{ fontFamily: display, fontWeight: 800, fontSize: 44, letterSpacing: '-0.04em' }}>Slimlytics</span>
        </div>

        <div style={{ marginTop: 50, width: 680, fontFamily: display, fontWeight: 800, fontSize: 74, lineHeight: 1.04, letterSpacing: '-0.045em' }}>
          Know what works.
          <br />
          <span
            style={{
              background: `linear-gradient(100deg, ${colors.mint}, ${colors.glow} 45%, ${colors.blue})`,
              WebkitBackgroundClip: 'text',
              backgroundClip: 'text',
              color: 'transparent'
            }}
          >
            Skip the noise.
          </span>
        </div>
        <p style={{ margin: '24px 0 0', width: 560, fontSize: 28, lineHeight: 1.4, color: colors.muted }}>
          Privacy-first, open-source web analytics. Self-host or use our cloud.
        </p>

        <div style={{ position: 'absolute', left: 72, bottom: 60, display: 'flex', gap: 12 }}>
          {['Cookieless', 'Real-time', 'AI-agent setup', 'Open source'].map((tag) => (
            <span
              key={tag}
              style={{
                padding: '10px 18px',
                borderRadius: 999,
                border: `1.5px solid rgba(52,211,153,0.35)`,
                background: 'rgba(52,211,153,0.09)',
                fontSize: 21,
                fontWeight: 600
              }}
            >
              {tag}
            </span>
          ))}
        </div>

        <div
          style={{
            position: 'absolute',
            right: 34,
            top: 112,
            width: 400,
            padding: 26,
            borderRadius: 26,
            background: 'linear-gradient(180deg, rgba(23,33,48,0.96), rgba(12,19,29,0.98))',
            border: '1px solid rgba(255,255,255,0.12)',
            boxShadow: '0 50px 100px -30px rgba(0,0,0,0.9), 0 0 0 1px rgba(52,211,153,0.15)',
            transform: 'rotate(-3deg)'
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: 9, padding: '6px 14px', borderRadius: 999, background: 'rgba(52,211,153,0.13)', color: colors.glow, fontWeight: 700, fontSize: 17 }}>
              <span style={{ width: 9, height: 9, borderRadius: 9, background: colors.glow, boxShadow: '0 0 0 5px rgba(52,211,153,0.2)' }} />
              Live
            </span>
            <span style={{ fontSize: 17, color: colors.muted }}>
              <b style={{ fontFamily: display, fontSize: 26, color: colors.text }}>27</b> online
            </span>
          </div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginTop: 18 }}>
            <span style={{ fontFamily: display, fontWeight: 800, fontSize: 48, letterSpacing: '-0.03em' }}>48.2K</span>
            <span style={{ fontSize: 18, color: colors.muted }}>visitors</span>
            <span style={{ marginLeft: 'auto', fontSize: 17, fontWeight: 700, color: colors.glow, background: 'rgba(52,211,153,0.13)', padding: '5px 11px', borderRadius: 999 }}>▲ 18%</span>
          </div>
          <svg viewBox="0 0 380 120" style={{ width: '100%', height: 120, marginTop: 10, overflow: 'visible' }}>
            <defs>
              <linearGradient id="og-fill" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stopColor={colors.glow} stopOpacity={0.35} />
                <stop offset="1" stopColor={colors.glow} stopOpacity={0} />
              </linearGradient>
            </defs>
            <path d={`${line} L380,120 L0,120 Z`} fill="url(#og-fill)" />
            <path d={line} fill="none" stroke={colors.glow} strokeWidth={4} strokeLinecap="round" />
          </svg>
          <div style={{ display: 'grid', gap: 9, marginTop: 16 }}>
            {feed.map((item) => (
              <div key={item.page} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '9px 12px', borderRadius: 12, background: 'rgba(255,255,255,0.04)', fontSize: 19 }}>
                <span style={{ fontSize: 24 }}>{item.flag}</span>
                <b style={{ flex: 1 }}>{item.page}</b>
                <span style={{ color: colors.muted, fontSize: 16 }}>{item.place}</span>
              </div>
            ))}
          </div>
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}
