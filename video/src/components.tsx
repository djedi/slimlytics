import type { CSSProperties, ReactNode } from 'react';
import { AbsoluteFill, Easing, interpolate, spring, useCurrentFrame, useVideoConfig } from 'remotion';
import { body, colors, display } from './theme';

/** Brand backdrop: ink with slowly drifting glows and a faded grid. */
export function Backdrop({ hue = 'green' }: { hue?: 'green' | 'blue' }) {
  const frame = useCurrentFrame();
  const drift = Math.sin(frame / 60) * 60;
  const primary = hue === 'green' ? 'rgba(16,185,129,0.28)' : 'rgba(122,162,255,0.24)';
  const secondary = hue === 'green' ? 'rgba(122,162,255,0.16)' : 'rgba(16,185,129,0.18)';
  return (
    <AbsoluteFill style={{ background: colors.ink, overflow: 'hidden' }}>
      <AbsoluteFill
        style={{
          background: `radial-gradient(900px 600px at ${70 + drift / 20}% ${30 + drift / 30}%, ${primary}, transparent 70%),
            radial-gradient(800px 500px at ${10 - drift / 30}% ${90}%, ${secondary}, transparent 70%)`
        }}
      />
      <AbsoluteFill
        style={{
          backgroundImage: `linear-gradient(${colors.line} 1px, transparent 1px), linear-gradient(90deg, ${colors.line} 1px, transparent 1px)`,
          backgroundSize: '64px 64px',
          backgroundPosition: `${frame * 0.4}px ${frame * 0.2}px`,
          maskImage: 'radial-gradient(ellipse 70% 65% at 50% 45%, #000 20%, transparent 75%)',
          WebkitMaskImage: 'radial-gradient(ellipse 70% 65% at 50% 45%, #000 20%, transparent 75%)'
        }}
      />
    </AbsoluteFill>
  );
}

/** Spring progress starting at `delay` frames into the current sequence. */
export function useEnter(delay = 0, damping = 18) {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  return spring({ frame: frame - delay, fps, config: { damping, mass: 0.8 } });
}

export function Rise({ delay = 0, children, style }: { delay?: number; children: ReactNode; style?: CSSProperties }) {
  const progress = useEnter(delay);
  return (
    <div
      style={{
        opacity: progress,
        transform: `translateY(${interpolate(progress, [0, 1], [36, 0])}px)`,
        ...style
      }}
    >
      {children}
    </div>
  );
}

/** Headline revealed word by word. Words wrapped in *asterisks* get the brand gradient. */
export function Headline({ text, size = 92, delay = 0, align = 'center' }: { text: string; size?: number; delay?: number; align?: 'center' | 'left' }) {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const words = text.split(' ');
  return (
    <h1
      style={{
        margin: 0,
        fontFamily: display,
        fontWeight: 800,
        fontSize: size,
        lineHeight: 1.04,
        letterSpacing: '-0.045em',
        color: colors.text,
        textAlign: align,
        display: 'flex',
        flexWrap: 'wrap',
        justifyContent: align === 'center' ? 'center' : 'flex-start',
        columnGap: size * 0.26
      }}
    >
      {words.map((word, index) => {
        const progress = spring({ frame: frame - delay - index * 3, fps, config: { damping: 16 } });
        const highlighted = word.startsWith('*');
        const clean = word.replace(/\*/g, '');
        // Spread one gradient across the whole highlighted run instead of restarting per word.
        const run = words.filter((w) => w.startsWith('*'));
        const position = run.indexOf(word);
        const span = Math.max(1, run.length);
        return (
          <span
            key={index}
            style={{
              display: 'inline-block',
              opacity: progress,
              transform: `translateY(${interpolate(progress, [0, 1], [size * 0.5, 0])}px)`,
              ...(highlighted
                ? {
                    background: `linear-gradient(100deg, ${colors.mint}, ${colors.glow} 45%, ${colors.blue})`,
                    backgroundSize: `${span * 100}% 100%`,
                    backgroundPosition: `${span > 1 ? (position / (span - 1)) * 100 : 0}% 0`,
                    WebkitBackgroundClip: 'text',
                    backgroundClip: 'text',
                    color: 'transparent'
                  }
                : {})
            }}
          >
            {clean}
          </span>
        );
      })}
    </h1>
  );
}

export function Kicker({ children, delay = 0 }: { children: ReactNode; delay?: number }) {
  return (
    <Rise delay={delay}>
      <div
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 12,
          padding: '10px 20px',
          borderRadius: 999,
          border: `1px solid ${colors.line}`,
          background: 'rgba(255,255,255,0.04)',
          color: colors.glow,
          fontFamily: body,
          fontWeight: 700,
          fontSize: 22,
          letterSpacing: '0.14em',
          textTransform: 'uppercase'
        }}
      >
        {children}
      </div>
    </Rise>
  );
}

export function LogoMark({ size = 96, grow = 1 }: { size?: number; grow?: number }) {
  const bars = [0.42, 0.7, 1];
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: size * 0.27,
        background: `linear-gradient(140deg, #10b981, #047857)`,
        boxShadow: `0 ${size * 0.25}px ${size * 0.6}px -${size * 0.2}px rgba(16,185,129,0.7), inset 0 1px 0 rgba(255,255,255,0.3)`,
        display: 'flex',
        alignItems: 'flex-end',
        justifyContent: 'center',
        gap: size * 0.08,
        padding: size * 0.24
      }}
    >
      {bars.map((height, index) => (
        <div
          key={index}
          style={{
            width: size * 0.13,
            height: `${height * 100 * Math.min(1, Math.max(0, grow * 1.6 - index * 0.3))}%`,
            borderRadius: size * 0.03,
            background: '#fff'
          }}
        />
      ))}
    </div>
  );
}

export function Panel({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return (
    <div
      style={{
        background: 'linear-gradient(180deg, rgba(23,33,48,0.94), rgba(12,19,29,0.97))',
        border: `1px solid rgba(255,255,255,0.1)`,
        borderRadius: 28,
        boxShadow: '0 60px 120px -40px rgba(0,0,0,0.85)',
        color: colors.text,
        fontFamily: body,
        ...style
      }}
    >
      {children}
    </div>
  );
}

export function Chip({ children, delay = 0, tone = 'green' }: { children: ReactNode; delay?: number; tone?: 'green' | 'blue' | 'amber' }) {
  const color = tone === 'green' ? colors.glow : tone === 'blue' ? colors.blue : colors.amber;
  const progress = useEnter(delay, 14);
  return (
    <div
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 14,
        padding: '18px 28px',
        borderRadius: 999,
        border: `1.5px solid ${color}55`,
        background: `${color}14`,
        color: colors.text,
        fontFamily: body,
        fontWeight: 600,
        fontSize: 32,
        opacity: progress,
        transform: `scale(${interpolate(progress, [0, 1], [0.85, 1])})`
      }}
    >
      {children}
    </div>
  );
}

/** Count from 0 to `to` with an ease-out, formatted by `format`. */
export function useCount(to: number, start: number, duration: number) {
  const frame = useCurrentFrame();
  return interpolate(frame, [start, start + duration], [0, to], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
    easing: Easing.out(Easing.cubic)
  });
}

export const Check = ({ color = colors.glow, size = 34 }: { color?: string; size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={3} strokeLinecap="round" strokeLinejoin="round">
    <path d="M20 6 9 17l-5-5" />
  </svg>
);
