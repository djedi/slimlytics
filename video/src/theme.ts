import { loadFont as loadInter } from '@remotion/google-fonts/Inter';
import { loadFont as loadManrope } from '@remotion/google-fonts/Manrope';

// Same palette as the marketing site (frontend/src/lib/marketing/marketing.css).
export const colors = {
  ink: '#070c14',
  ink2: '#0e1622',
  panel: '#111b28',
  line: 'rgba(255,255,255,0.09)',
  text: '#eef3f8',
  muted: '#a3b0c2',
  glow: '#34d399',
  accent: '#059669',
  mint: '#6ee7b7',
  blue: '#7aa2ff',
  amber: '#f5b544',
  red: '#f87171'
};

export const display = loadManrope('normal', { weights: ['700', '800'], subsets: ['latin'] }).fontFamily;
export const body = loadInter('normal', { weights: ['400', '500', '600', '700'], subsets: ['latin'] }).fontFamily;
export const mono = 'ui-monospace, "SF Mono", Menlo, monospace';

export { FPS, TRANSITION } from './timeline';
