import { AbsoluteFill } from 'remotion';
import { Backdrop, LogoMark } from './components';
import { body, colors, display } from './theme';

/** Static poster for the homepage player: text above and below, open centre for the play button. */
export function Poster() {
  const tags = ['Privacy-first', 'Real-time', 'AI-agent native', 'Open source'];
  return (
    <AbsoluteFill>
      <Backdrop />
      <AbsoluteFill style={{ alignItems: 'center', justifyContent: 'space-between', flexDirection: 'column', padding: '120px 0 130px' }}>
        <div style={{ display: 'grid', justifyItems: 'center', gap: 34 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 24 }}>
            <LogoMark size={88} />
            <span style={{ fontFamily: display, fontWeight: 800, fontSize: 64, letterSpacing: '-0.04em', color: colors.text }}>Slimlytics</span>
          </div>
          <div style={{ fontFamily: display, fontWeight: 800, fontSize: 104, letterSpacing: '-0.045em', color: colors.text, lineHeight: 1 }}>
            Know what works.{' '}
            <span style={{ background: `linear-gradient(100deg, ${colors.mint}, ${colors.glow} 45%, ${colors.blue})`, WebkitBackgroundClip: 'text', backgroundClip: 'text', color: 'transparent' }}>
              Skip the noise.
            </span>
          </div>
        </div>
        <div style={{ display: 'flex', gap: 18 }}>
          {tags.map((tag) => (
            <span key={tag} style={{ padding: '14px 26px', borderRadius: 999, border: `1px solid ${colors.line}`, background: 'rgba(255,255,255,0.05)', color: colors.text, fontFamily: body, fontWeight: 600, fontSize: 30 }}>
              {tag}
            </span>
          ))}
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}
