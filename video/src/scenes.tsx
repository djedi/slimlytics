import { AbsoluteFill, Easing, interpolate, useCurrentFrame } from 'remotion';
import { Backdrop, Check, Chip, Headline, Kicker, LogoMark, Panel, Rise, useCount, useEnter } from './components';
import { body, colors, display, mono } from './theme';

const center: React.CSSProperties = { alignItems: 'center', justifyContent: 'center', flexDirection: 'column' };

/* 1 — Hook */
export function Intro() {
  const frame = useCurrentFrame();
  const logo = useEnter(0, 14);
  const grow = interpolate(frame, [4, 30], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp', easing: Easing.out(Easing.cubic) });
  const word = useEnter(14);
  return (
    <AbsoluteFill>
      <Backdrop />
      <AbsoluteFill style={{ ...center, gap: 54 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 30, transform: `scale(${interpolate(logo, [0, 1], [0.6, 1])})`, opacity: logo }}>
          <LogoMark size={128} grow={grow} />
          <span
            style={{
              fontFamily: display,
              fontWeight: 800,
              fontSize: 96,
              letterSpacing: '-0.04em',
              color: colors.text,
              opacity: word,
              transform: `translateX(${interpolate(word, [0, 1], [-30, 0])}px)`
            }}
          >
            Slimlytics
          </span>
        </div>
        <div style={{ display: 'grid', gap: 6 }}>
          <Headline text="Know what works." size={124} delay={34} />
          <Headline text="*Skip* *the* *noise.*" size={124} delay={46} />
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}

/* 2 — Privacy */
export function Privacy() {
  const frame = useCurrentFrame();
  const strike = interpolate(frame, [26, 44], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp', easing: Easing.out(Easing.cubic) });
  const cookie = useEnter(8, 12);
  return (
    <AbsoluteFill>
      <Backdrop hue="blue" />
      <AbsoluteFill style={{ ...center, gap: 52, padding: 120 }}>
        <Kicker>Privacy-first analytics</Kicker>
        <Headline text="All the insight. *None* *of* *the* *creepy.*" size={100} delay={6} />
        <div style={{ display: 'flex', alignItems: 'center', gap: 60 }}>
          <div style={{ position: 'relative', width: 150, height: 150, opacity: cookie, transform: `rotate(${interpolate(cookie, [0, 1], [-30, 0])}deg)` }}>
            <svg viewBox="0 0 24 24" width={150} height={150} fill="none" strokeLinecap="round" strokeLinejoin="round">
              <g stroke={colors.amber} strokeWidth={1.6}>
                <path d="M12 2a10 10 0 1 0 10 10 4 4 0 0 1-5-5 4 4 0 0 1-5-5" />
                <path d="M8.5 8.5v.01M16 15.5v.01M12 12v.01M11 17v.01M7 14v.01" />
              </g>
              {/* Strike drawn in the cookie's own coordinates, corner to corner through its centre. */}
              <path d="M3 21 21 3" stroke={colors.ink} strokeWidth={4.4} pathLength={1} strokeDasharray={1} strokeDashoffset={1 - strike} />
              <path d="M3 21 21 3" stroke={colors.red} strokeWidth={2.2} pathLength={1} strokeDasharray={1} strokeDashoffset={1 - strike} />
            </svg>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, auto)', gap: 20 }}>
            <Chip delay={30}><Check /> No cookies</Chip>
            <Chip delay={36}><Check /> No fingerprinting</Chip>
            <Chip delay={42}><Check /> No session replay</Chip>
            <Chip delay={48}><Check /> Respects DNT &amp; GPC</Chip>
          </div>
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}

/* 3 — Live Spy */
const stream = [
  { flag: '🇺🇸', place: 'Austin, United States', page: '/pricing', kind: 'pageview', at: 12 },
  { flag: '🇩🇪', place: 'Berlin, Germany', page: '/docs/mcp', kind: 'pageview', at: 30 },
  { flag: '🇯🇵', place: 'Tokyo, Japan', page: 'signup', kind: 'goal', at: 48 },
  { flag: '🇬🇧', place: 'London, United Kingdom', page: '/blog/launch', kind: 'pageview', at: 66 },
  { flag: '🇧🇷', place: 'São Paulo, Brazil', page: '/', kind: 'pageview', at: 84 },
  { flag: '🇨🇦', place: 'Toronto, Canada', page: 'download', kind: 'goal', at: 102 }
];

export function Spy() {
  const frame = useCurrentFrame();
  const active = Math.round(useCount(27, 10, 110));
  const visible = stream.filter((item) => frame >= item.at).reverse();
  const bars = [3, 5, 4, 7, 6, 9, 8, 12, 10, 14, 11, 16, 13, 18, 15, 21, 17, 24];
  return (
    <AbsoluteFill>
      <Backdrop />
      <AbsoluteFill style={{ flexDirection: 'row', alignItems: 'center', gap: 90, padding: '0 130px' }}>
        <div style={{ flex: '0 0 640px', display: 'grid', gap: 34 }}>
          <Kicker>Real-time Spy</Kicker>
          <Headline text="Watch visitors arrive. *Live.*" size={96} align="left" delay={4} />
          <Rise delay={24}>
            <p style={{ margin: 0, fontFamily: body, fontSize: 34, lineHeight: 1.45, color: colors.muted }}>
              Every page view and goal streams in the moment it happens.
            </p>
          </Rise>
        </div>
        <Rise delay={8} style={{ flex: 1 }}>
          <Panel style={{ padding: 36, display: 'grid', gap: 26 }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 12, padding: '10px 20px', borderRadius: 999, background: 'rgba(52,211,153,0.12)', color: colors.glow, fontWeight: 700, fontSize: 24 }}>
                <span style={{ width: 12, height: 12, borderRadius: 999, background: colors.glow, boxShadow: `0 0 0 ${6 + Math.sin(frame / 4) * 4}px rgba(52,211,153,0.25)` }} />
                Live
              </span>
              <span style={{ fontSize: 24, color: colors.muted }}>
                <b style={{ color: colors.text, fontFamily: display, fontSize: 40 }}>{active}</b> active now
              </span>
            </div>
            <div style={{ display: 'flex', alignItems: 'flex-end', gap: 7, height: 90 }}>
              {bars.map((value, index) => {
                const grow = interpolate(frame, [10 + index * 3, 26 + index * 3], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
                return <div key={index} style={{ flex: 1, height: `${(value / 24) * 100 * grow}%`, borderRadius: 6, background: `linear-gradient(180deg, ${colors.glow}, rgba(52,211,153,0.35))` }} />;
              })}
            </div>
            <div style={{ display: 'grid', gap: 12, height: 420, overflow: 'hidden' }}>
              {visible.map((item) => {
                const enter = interpolate(frame, [item.at, item.at + 10], [0, 1], { extrapolateRight: 'clamp', easing: Easing.out(Easing.cubic) });
                const goal = item.kind === 'goal';
                return (
                  <div
                    key={item.place}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 18,
                      padding: '14px 18px',
                      borderRadius: 16,
                      background: enter < 1 ? `rgba(52,211,153,${0.16 * (1 - enter)})` : 'rgba(255,255,255,0.03)',
                      opacity: enter,
                      transform: `translateY(${(1 - enter) * -24}px)`
                    }}
                  >
                    <span style={{ fontSize: 40 }}>{item.flag}</span>
                    <span style={{ flex: 1, display: 'grid' }}>
                      <b style={{ fontSize: 28 }}>{goal ? `🎯 ${item.page}` : item.page}</b>
                      <span style={{ fontSize: 22, color: colors.muted }}>{item.place}</span>
                    </span>
                    <span style={{ fontSize: 22, color: goal ? colors.amber : colors.glow, fontWeight: 600 }}>{goal ? 'goal' : 'just now'}</span>
                  </div>
                );
              })}
            </div>
          </Panel>
        </Rise>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}

/* 4 — Dashboard */
function smoothPath(values: number[], width: number, height: number) {
  const max = Math.max(...values);
  const points = values.map((value, index) => [(index / (values.length - 1)) * width, height - (value / max) * height * 0.92]);
  return points.reduce((path, [x, y], index) => {
    if (!index) return `M${x},${y}`;
    const [px, py] = points[index - 1];
    const mid = (px + x) / 2;
    return `${path} C${mid},${py} ${mid},${y} ${x},${y}`;
  }, '');
}

export function Dashboard() {
  const frame = useCurrentFrame();
  const visitors = useCount(48.2, 14, 60);
  const views = useCount(132, 20, 60);
  const bounce = useCount(38, 26, 60);
  const draw = interpolate(frame, [24, 100], [1, 0], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp', easing: Easing.inOut(Easing.cubic) });
  const trend = [22, 28, 25, 34, 31, 42, 39, 48, 45, 57, 53, 66, 61, 74, 70, 82, 79, 94];
  const path = smoothPath(trend, 1000, 300);
  const metrics = [
    { label: 'Visitors', value: `${visitors.toFixed(1)}K`, badge: '▲ 18%' },
    { label: 'Page views', value: `${Math.round(views)}K`, badge: '▲ 24%' },
    { label: 'Bounce rate', value: `${Math.round(bounce)}%`, badge: '▼ 6 pts' },
    { label: 'Avg. visit', value: '2m 41s', badge: '▲ 11%' }
  ];
  return (
    <AbsoluteFill>
      <Backdrop hue="blue" />
      <AbsoluteFill style={{ ...center, gap: 44, padding: '0 130px' }}>
        <div style={{ display: 'grid', gap: 4 }}>
          <Headline text="Every number that matters." size={84} delay={2} />
          <Headline text="*Nothing* *that* *doesn’t.*" size={84} delay={14} />
        </div>
        <Rise delay={10} style={{ width: '100%' }}>
          <Panel style={{ padding: 40, display: 'grid', gap: 30 }}>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 22 }}>
              {metrics.map((metric, index) => (
                <Rise key={metric.label} delay={14 + index * 5}>
                  <div style={{ padding: '22px 26px', borderRadius: 20, background: 'rgba(255,255,255,0.035)', border: `1px solid ${colors.line}` }}>
                    <div style={{ fontSize: 24, color: colors.muted, fontWeight: 600 }}>{metric.label}</div>
                    <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', marginTop: 8 }}>
                      <span style={{ fontFamily: display, fontWeight: 800, fontSize: 60, letterSpacing: '-0.03em' }}>{metric.value}</span>
                      <span style={{ fontSize: 22, fontWeight: 700, color: colors.glow, background: 'rgba(52,211,153,0.13)', padding: '6px 12px', borderRadius: 999 }}>{metric.badge}</span>
                    </div>
                  </div>
                </Rise>
              ))}
            </div>
            <svg viewBox="0 0 1000 300" preserveAspectRatio="none" style={{ width: '100%', height: 300, overflow: 'visible' }}>
              <defs>
                <linearGradient id="fill" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0" stopColor={colors.glow} stopOpacity={0.32} />
                  <stop offset="1" stopColor={colors.glow} stopOpacity={0} />
                </linearGradient>
              </defs>
              {[0, 1, 2, 3].map((row) => (
                <line key={row} x1={0} x2={1000} y1={row * 100} y2={row * 100} stroke={colors.line} strokeDasharray="4 6" vectorEffect="non-scaling-stroke" />
              ))}
              <path d={`${path} L1000,300 L0,300 Z`} fill="url(#fill)" opacity={1 - draw} />
              <path d={path} fill="none" stroke={colors.glow} strokeWidth={5} strokeLinecap="round" pathLength={1} strokeDasharray={1} strokeDashoffset={draw} vectorEffect="non-scaling-stroke" />
            </svg>
          </Panel>
        </Rise>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}

/* 5 — AI agents */
const command = 'claude mcp add --transport http slimlytics \\\n    https://slimlytics.com/api/mcp';
const ask = 'Add Slimlytics to my site and verify tracking.';
const steps = [
  { label: 'slimlytics · setup_site', detail: 'shop.example.com · nginx', at: 96 },
  { label: 'Edit', detail: 'nginx/site.conf  +14 lines', at: 108 },
  { label: 'Edit', detail: 'src/app.html  +1 line', at: 118 },
  { label: 'Verify', detail: 'script 200 · beacon 200', at: 128 }
];

export function Agents() {
  const frame = useCurrentFrame();
  const typed = Math.floor(interpolate(frame, [12, 50], [0, command.length], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' }));
  const asked = Math.floor(interpolate(frame, [58, 86], [0, ask.length], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' }));
  const done = useEnter(140, 12);
  const caret = frame % 16 < 8 ? '▍' : ' ';
  return (
    <AbsoluteFill>
      <Backdrop />
      <AbsoluteFill style={{ flexDirection: 'row', alignItems: 'center', gap: 80, padding: '0 120px' }}>
        <div style={{ flex: '0 0 620px', display: 'grid', gap: 34 }}>
          <Kicker>AI-agent native</Kicker>
          <Headline text="Just ask your *AI* *agent* to install it." size={88} align="left" delay={4} />
          <Rise delay={30} style={{ display: 'flex', flexWrap: 'wrap', gap: 14 }}>
            {['Claude Code', 'Codex', 'Hermes', 'Any MCP client'].map((name) => (
              <span key={name} style={{ padding: '10px 18px', borderRadius: 999, border: `1px solid ${colors.line}`, background: 'rgba(255,255,255,0.04)', color: colors.text, fontFamily: body, fontWeight: 600, fontSize: 24 }}>
                {name}
              </span>
            ))}
          </Rise>
        </div>
        <Rise delay={6} style={{ flex: 1 }}>
          <div style={{ borderRadius: 24, overflow: 'hidden', border: '1px solid rgba(255,255,255,0.12)', background: '#05080e', boxShadow: '0 60px 120px -40px rgba(0,0,0,0.9)', fontFamily: mono }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '18px 24px', borderBottom: '1px solid rgba(255,255,255,0.08)', color: colors.muted, fontSize: 20 }}>
              <i style={{ width: 14, height: 14, borderRadius: 99, background: '#ff5f57' }} />
              <i style={{ width: 14, height: 14, borderRadius: 99, background: '#febc2e' }} />
              <i style={{ width: 14, height: 14, borderRadius: 99, background: '#28c840' }} />
              <span style={{ marginLeft: 12 }}>~/shop — claude</span>
            </div>
            <div style={{ padding: '30px 34px', fontSize: 26, lineHeight: 1.65, color: '#dbe4ee', minHeight: 560 }}>
              <div style={{ whiteSpace: 'pre-wrap' }}>
                <span style={{ color: colors.glow }}>$ </span>
                {command.slice(0, typed)}
                {frame < 54 ? caret : ''}
              </div>
              {frame > 52 && <div style={{ color: colors.muted }}>✓ Connected · browser login complete</div>}
              {frame > 56 && (
                <div style={{ marginTop: 22 }}>
                  <span style={{ color: colors.blue }}>&gt; </span>
                  {ask.slice(0, asked)}
                  {frame >= 56 && frame < 92 ? caret : ''}
                </div>
              )}
              <div style={{ marginTop: 18, display: 'grid', gap: 6 }}>
                {steps.map((step) =>
                  frame >= step.at ? (
                    <div key={step.detail} style={{ opacity: interpolate(frame, [step.at, step.at + 6], [0, 1], { extrapolateRight: 'clamp' }) }}>
                      <span style={{ color: '#b8c9ff' }}>● {step.label}</span> <span style={{ color: '#8593a6' }}>{step.detail}</span>
                    </div>
                  ) : null
                )}
              </div>
              <div
                style={{
                  marginTop: 24,
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 14,
                  padding: '14px 22px',
                  borderRadius: 14,
                  background: 'rgba(52,211,153,0.13)',
                  color: colors.mint,
                  fontWeight: 700,
                  opacity: done,
                  transform: `scale(${interpolate(done, [0, 1], [0.9, 1])})`
                }}
              >
                <Check color={colors.mint} size={30} /> Tracking live. First visit recorded.
              </div>
            </div>
          </div>
        </Rise>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}

/* 6 — Open source, self-host or cloud */
export function Deploy() {
  const features = ['Real-time Spy', 'Goals & funnels', 'Attribution', 'First-party proxy', 'CLI & REST API', 'Multi-site'];
  return (
    <AbsoluteFill>
      <Backdrop hue="blue" />
      <AbsoluteFill style={{ ...center, gap: 50, padding: '0 150px' }}>
        <Headline text="*Open* *source.* Run it your way." size={100} delay={2} />
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 36, width: '100%' }}>
          <Rise delay={14}>
            <Panel style={{ padding: 44, display: 'grid', gap: 18, height: '100%' }}>
              <div style={{ fontSize: 26, fontWeight: 700, color: colors.glow, letterSpacing: '0.1em', textTransform: 'uppercase' }}>Self-host</div>
              <div style={{ fontFamily: display, fontWeight: 800, fontSize: 52, letterSpacing: '-0.03em' }}>Your servers. Your data.</div>
              <div style={{ fontFamily: mono, fontSize: 28, color: '#dbe4ee', background: '#05080e', borderRadius: 14, padding: '16px 22px', border: `1px solid ${colors.line}` }}>
                <span style={{ color: colors.glow }}>$</span> docker compose up -d
              </div>
            </Panel>
          </Rise>
          <Rise delay={22}>
            <Panel style={{ padding: 44, display: 'grid', gap: 18, height: '100%', boxShadow: '0 0 0 1.5px rgba(52,211,153,0.5), 0 60px 120px -40px rgba(5,150,105,0.5)' }}>
              <div style={{ fontSize: 26, fontWeight: 700, color: colors.blue, letterSpacing: '0.1em', textTransform: 'uppercase' }}>Slimlytics Cloud</div>
              <div style={{ fontFamily: display, fontWeight: 800, fontSize: 52, letterSpacing: '-0.03em' }}>Or let us run it.</div>
              <div style={{ fontSize: 30, color: colors.muted, lineHeight: 1.4 }}>Sign up and add a site in minutes. No servers to manage.</div>
            </Panel>
          </Rise>
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap', justifyContent: 'center', gap: 16 }}>
          {features.map((feature, index) => (
            <Chip key={feature} delay={36 + index * 5} tone={index % 2 ? 'blue' : 'green'}>
              {feature}
            </Chip>
          ))}
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}

/* 7 — Call to action */
export function CallToAction() {
  const frame = useCurrentFrame();
  const logo = useEnter(0, 14);
  const button = useEnter(26, 12);
  const pulse = 1 + Math.max(0, Math.sin((frame - 50) / 7)) * 0.025;
  return (
    <AbsoluteFill>
      <Backdrop />
      <AbsoluteFill style={{ ...center, gap: 44 }}>
        <div style={{ opacity: logo, transform: `scale(${interpolate(logo, [0, 1], [0.7, 1])})` }}>
          <LogoMark size={120} />
        </div>
        <Headline text="Analytics you’ll *actually* *love.*" size={110} delay={6} />
        <div
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: 18,
            padding: '30px 56px',
            borderRadius: 999,
            background: `linear-gradient(140deg, #10b981, #047857)`,
            color: '#fff',
            fontFamily: body,
            fontWeight: 700,
            fontSize: 46,
            boxShadow: '0 30px 70px -20px rgba(16,185,129,0.75)',
            opacity: button,
            transform: `scale(${interpolate(button, [0, 1], [0.8, 1]) * (frame > 50 ? pulse : 1)})`
          }}
        >
          Start free at slimlytics.com →
        </div>
        <Rise delay={40}>
          <div style={{ fontFamily: body, fontSize: 30, color: colors.muted }}>Open source · Self-host or cloud · Privacy-first</div>
        </Rise>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}
