// Synthesizes the promo soundtrack (music + sound effects) from scratch and writes
// public/promo-audio.wav. Everything here is original, generated audio, so the
// open-source repo carries no third-party music licences.
//
// Effects are scheduled from src/timeline.ts, so they stay aligned with the visuals.
// Usage: node scripts/compose-audio.mjs
import { mkdirSync, writeFileSync } from 'node:fs';
import { FPS, PROMO_FRAMES, sceneStart, SCENES, TRANSITION } from '../src/timeline.ts';

const SR = 48000;
const DURATION = PROMO_FRAMES / FPS;
const LENGTH = Math.round(DURATION * SR);
const TAU = Math.PI * 2;

// Busses: dry music, dry effects, and a reverb send shared by both.
const bus = () => [new Float32Array(LENGTH), new Float32Array(LENGTH)];
const music = bus();
const sfx = bus();
const send = bus();

// Deterministic randomness so every render sounds identical.
let seed = 0x5eed;
const random = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296);

const at = (scene, frame) => (sceneStart(scene) + frame) / FPS;
const hz = (midi) => 440 * 2 ** ((midi - 69) / 12);

/** Mix a mono voice into a bus with equal-power panning and an optional reverb send. */
function write(target, start, seconds, voice, { gain = 1, pan = 0, reverb = 0 } = {}) {
  const first = Math.max(0, Math.round(start * SR));
  const last = Math.min(LENGTH, first + Math.round(seconds * SR));
  const left = Math.cos(((pan + 1) * Math.PI) / 4) * gain;
  const right = Math.sin(((pan + 1) * Math.PI) / 4) * gain;
  for (let i = first; i < last; i++) {
    const value = voice((i - first) / SR);
    target[0][i] += value * left;
    target[1][i] += value * right;
    if (reverb) {
      send[0][i] += value * left * reverb;
      send[1][i] += value * right * reverb;
    }
  }
}

const envelope = (t, length, attack, release) =>
  Math.min(1, t / attack) * Math.min(1, Math.max(0, (length - t) / release));
const decay = (t, rate) => Math.exp(-t * rate);

// ---------------------------------------------------------------- music
// 120 BPM in C major. Chord changes build toward a resolution on C as the call to action lands.
const BEAT = 0.5;
const ctaTime = at('cta', 0);
const chords = [
  { start: 0, notes: [48, 52, 55, 60] }, // C
  { start: 4, notes: [45, 52, 57, 60] }, // Am
  { start: 8, notes: [41, 53, 57, 60] }, // F
  { start: 12, notes: [43, 55, 59, 62] }, // G
  { start: 16, notes: [48, 52, 55, 64] }, // C
  { start: 20, notes: [45, 52, 57, 64] }, // Am
  { start: 22, notes: [41, 53, 57, 65] }, // F
  { start: 24, notes: [43, 55, 59, 62] }, // G (build)
  { start: ctaTime, notes: [36, 48, 55, 60, 64, 67] } // C (resolve)
];
const chordAt = (time) => [...chords].reverse().find((chord) => time >= chord.start - 1e-6);

// Warm pad: slightly detuned, softly filtered saws (additive, so no aliasing).
function pad(freq, t, length) {
  let value = 0;
  for (const detune of [-0.06, 0.07]) {
    const f = freq * 2 ** (detune / 12);
    for (let h = 1; h <= 7; h++) value += Math.sin(TAU * f * h * t) / (h * h * 0.8);
  }
  return value * envelope(t, length, 0.7, 1.2) * 0.05;
}
chords.forEach((chord, index) => {
  const end = chords[index + 1]?.start ?? DURATION;
  const length = end - chord.start + (index === chords.length - 1 ? 0 : 0.8);
  chord.notes.forEach((note, voice) => {
    const pan = (voice / (chord.notes.length - 1)) * 1.2 - 0.6;
    const final = index === chords.length - 1;
    write(music, chord.start, length, (t) => pad(hz(note), t, length) * (final ? 1.25 : 1), { pan, reverb: 0.5 });
  });
});

// Kick on every beat from the privacy scene until the resolve, with a final hit on the resolve.
const kickTimes = [];
for (let t = at('privacy', 0); t < ctaTime - 0.01; t += BEAT) kickTimes.push(t);
kickTimes.push(ctaTime);
function kick(t) {
  const freq = 45 + 110 * decay(t, 28);
  return Math.sin(TAU * freq * t - 3 * decay(t, 28)) * decay(t, 8) * 0.45;
}
for (const time of kickTimes) write(music, time, 0.5, kick, { gain: time === ctaTime ? 1.3 : 1 });

// Sidechain-style ducking: other parts dip after each kick for a gentle pump.
const duck = (time) => {
  const since = (time - at('privacy', 0)) % BEAT;
  if (time < at('privacy', 0) || time >= ctaTime) return 1;
  return 0.55 + 0.45 * Math.min(1, since / 0.22);
};

// Bass: root note in eighths from the privacy scene.
for (let time = at('privacy', 0); time < ctaTime - 0.01; time += BEAT / 2) {
  const root = chordAt(time).notes[0];
  const freq = hz(root < 40 ? root + 12 : root) / 2;
  write(music, time, 0.24, (t) => (Math.sin(TAU * freq * t) + 0.3 * Math.sin(TAU * freq * 2 * t)) * envelope(t, 0.24, 0.005, 0.08) * 0.16 * duck(time + t));
}
write(music, ctaTime, 3.5, (t) => Math.sin(TAU * hz(24) * t) * envelope(t, 3.5, 0.01, 2.5) * 0.3);

// Hi-hats: off-beats from the Spy scene, sixteenths during the final build.
const noise = () => random() * 2 - 1;
function hat(t) {
  // Crude high-pass: difference of successive noise samples.
  return (noise() - noise()) * decay(t, 70) * 0.05;
}
for (let time = at('spy', 0) + BEAT / 2; time < ctaTime - 0.01; time += BEAT) write(music, time, 0.1, hat, { pan: 0.3 });
for (let time = 24; time < ctaTime - 0.01; time += BEAT / 4) write(music, time, 0.08, hat, { pan: -0.3, gain: 0.7 });

// Clap on beats two and four from the dashboard scene.
function clap(t) {
  const bursts = [0, 0.012, 0.024].reduce((sum, offset) => sum + (t >= offset ? decay(t - offset, 40) : 0), 0);
  return noise() * bursts * 0.06 + noise() * decay(t, 12) * 0.03;
}
for (let time = at('dashboard', 0) + BEAT; time < ctaTime - 0.01; time += BEAT * 2) write(music, time, 0.35, clap, { reverb: 0.3 });

// Arpeggio: plucked chord tones in sixteenths from the dashboard scene.
function pluck(freq) {
  return (t) => (Math.sin(TAU * freq * t) + 0.25 * Math.sin(TAU * freq * 2 * t + 0.5)) * decay(t, 14) * 0.06;
}
let step = 0;
for (let time = at('dashboard', 0); time < ctaTime - 0.01; time += BEAT / 4, step++) {
  const notes = chordAt(time).notes;
  const note = notes[[1, 2, 3, 2][step % 4]] + 12;
  write(music, time, 0.3, pluck(hz(note)), { pan: step % 2 ? 0.4 : -0.4, reverb: 0.35, gain: duck(time) });
}

// Build: a noise riser into the call to action.
const riserStart = ctaTime - 2;
write(music, riserStart, 2, (t) => noise() * (t / 2) ** 2 * 0.09 * (0.6 + 0.4 * Math.sin(TAU * (3 + t * 6) * t)), { reverb: 0.4 });
// Impact on the resolve: a soft crash.
write(music, ctaTime, 2.5, (t) => (noise() - noise()) * decay(t, 2.2) * 0.07, { reverb: 0.6 });

// ---------------------------------------------------------------- sound effects
function whoosh(time, length = 0.55, direction = 1) {
  let low = 0;
  write(
    sfx,
    time - length / 2,
    length,
    (t) => {
      const progress = t / length;
      // A one-pole low-pass whose cutoff sweeps up then down shapes the noise.
      const cutoff = 300 + 3200 * Math.sin(Math.PI * progress);
      const alpha = 1 - Math.exp((-TAU * cutoff) / SR);
      low += alpha * (noise() - low);
      return low * Math.sin(Math.PI * progress) ** 2 * 0.2;
    },
    { pan: direction * 0.35, reverb: 0.25 }
  );
}
const pop = (time, pitch = 1, gain = 1, pan = 0) =>
  write(sfx, time, 0.09, (t) => Math.sin(TAU * (520 + 520 * decay(t, 40)) * pitch * t) * decay(t, 45) * 0.22, { gain, pan, reverb: 0.15 });
const blip = (time, pan = 0) =>
  write(sfx, time, 0.16, (t) => (Math.sin(TAU * 1320 * t) + 0.4 * Math.sin(TAU * 2640 * t)) * decay(t, 32) * 0.1, { pan, reverb: 0.25 });
function bell(time, midi, gain = 1, pan = 0) {
  const freq = hz(midi);
  const partials = [[1, 1], [2.01, 0.45], [3.02, 0.22], [4.1, 0.12]];
  write(
    sfx,
    time,
    1.8,
    (t) => partials.reduce((sum, [ratio, level]) => sum + Math.sin(TAU * freq * ratio * t) * level * decay(t, 2.4 + ratio), 0) * 0.11,
    { gain, pan, reverb: 0.55 }
  );
}
const key = (time) =>
  write(sfx, time, 0.03, (t) => ((noise() - noise()) * 0.6 + Math.sin(TAU * 2900 * t) * 0.4) * decay(t, 260) * 0.12, {
    gain: 0.65 + random() * 0.5,
    pan: random() * 0.4 - 0.2
  });
const tick = (time) => write(sfx, time, 0.05, (t) => Math.sin(TAU * 2100 * t) * decay(t, 90) * 0.12, { reverb: 0.1 });
function sweep(time, length, from, to, gain = 1) {
  let phase = 0;
  write(
    sfx,
    time,
    length,
    (t) => {
      phase += (TAU * (from * (to / from) ** (t / length))) / SR;
      return Math.sin(phase) * Math.sin((Math.PI * t) / length) * 0.05;
    },
    { gain, reverb: 0.4 }
  );
}
function thud(time) {
  write(sfx, time, 0.25, (t) => Math.sin(TAU * (90 + 60 * decay(t, 30)) * t) * decay(t, 18) * 0.3);
}

// Scene transitions: a whoosh centred on each overlap, alternating direction.
SCENES.slice(1).forEach((scene, index) => whoosh(at(scene.id, TRANSITION / 2), 0.6, index % 2 ? -1 : 1));

// Intro: three rising plucks as the logo bars grow, then a shimmer as the headline lands.
[4, 12, 20].forEach((frame, index) => bell(at('intro', frame), [72, 76, 79][index], 0.8, index * 0.3 - 0.3));
sweep(at('intro', 44), 0.9, 600, 1800, 0.6);

// Privacy: cookie pops in, the strike swipes across, then a pop per badge.
pop(at('privacy', 8), 0.7, 1.1);
sweep(at('privacy', 26), 0.6, 300, 2400, 1.4);
[30, 36, 42, 48].forEach((frame, index) => pop(at('privacy', frame), 1 + index * 0.18, 0.9, index % 2 ? 0.3 : -0.3));

// Spy: a blip per visitor; goals get a brighter chime.
[[12, false], [30, false], [48, true], [66, false], [84, false], [102, true]].forEach(([frame, goal], index) => {
  const pan = index % 2 ? 0.35 : -0.35;
  if (goal) bell(at('spy', frame), 84, 0.7, pan);
  else blip(at('spy', frame), pan);
});

// Dashboard: ticks as the metrics land, and a rising tone while the chart draws.
[14, 19, 24, 29].forEach((frame) => tick(at('dashboard', frame)));
sweep(at('dashboard', 24), (100 - 24) / FPS, 220, 880, 1);

// Agents: keystrokes while typing, enter, a tick per tool step, and a success chime.
function typing(fromFrame, toFrame) {
  let time = at('agents', fromFrame);
  const end = at('agents', toFrame);
  while (time < end) {
    key(time);
    time += 0.045 + random() * 0.04;
  }
}
typing(12, 50);
write(sfx, at('agents', 52), 0.06, (t) => (noise() - noise()) * decay(t, 120) * 0.16);
typing(58, 86);
write(sfx, at('agents', 90), 0.06, (t) => (noise() - noise()) * decay(t, 120) * 0.16);
[96, 108, 118, 128].forEach((frame) => tick(at('agents', frame)));
[0, 0.09, 0.18].forEach((offset, index) => bell(at('agents', 140) + offset, [72, 76, 79][index], 0.9, index * 0.3 - 0.3));

// Deploy: soft thuds as the cards land, then a pop per feature chip.
[14, 22].forEach((frame) => thud(at('deploy', frame)));
[36, 41, 46, 51, 56, 61].forEach((frame, index) => pop(at('deploy', frame), 1.1 + (index % 3) * 0.15, 0.7, (index % 3) * 0.3 - 0.3));

// Call to action: a bright chime as the button lands.
[0, 0.07, 0.14, 0.21].forEach((offset, index) => bell(at('cta', 26) + offset, [79, 84, 88, 91][index], 0.75, index * 0.2 - 0.3));

// ---------------------------------------------------------------- reverb and mixdown
// Schroeder reverb on the send bus: parallel feedback combs into series all-passes.
function reverb(input, combs, allpasses) {
  const output = new Float32Array(LENGTH);
  for (const [delay, feedback] of combs) {
    const buffer = new Float32Array(delay);
    let index = 0;
    for (let i = 0; i < LENGTH; i++) {
      const delayed = buffer[index];
      buffer[index] = input[i] + delayed * feedback;
      output[i] += delayed / combs.length;
      index = (index + 1) % delay;
    }
  }
  for (const [delay, gain] of allpasses) {
    const buffer = new Float32Array(delay);
    let index = 0;
    for (let i = 0; i < LENGTH; i++) {
      const delayed = buffer[index];
      const value = output[i];
      buffer[index] = value + delayed * gain;
      output[i] = delayed - value * gain;
      index = (index + 1) % delay;
    }
  }
  return output;
}
const wetLeft = reverb(send[0], [[1557, 0.84], [1617, 0.83], [1491, 0.84], [1422, 0.85]], [[225, 0.5], [556, 0.5]]);
const wetRight = reverb(send[1], [[1580, 0.84], [1640, 0.83], [1514, 0.84], [1445, 0.85]], [[248, 0.5], [579, 0.5]]);

const mix = [new Float32Array(LENGTH), new Float32Array(LENGTH)];
for (let i = 0; i < LENGTH; i++) {
  const time = i / SR;
  // Fade in over 0.3 s and out over the last 1.4 s.
  const fade = Math.min(1, time / 0.3) * Math.min(1, (DURATION - time) / 1.4);
  mix[0][i] = (music[0][i] * 0.85 + sfx[0][i] + wetLeft[i] * 0.35) * fade;
  mix[1][i] = (music[1][i] * 0.85 + sfx[1][i] + wetRight[i] * 0.35) * fade;
}
let peak = 0;
for (const channel of mix) for (const value of channel) peak = Math.max(peak, Math.abs(value));
const scale = 0.89 / peak; // normalise to -1 dBFS; final loudness is set when publishing

// 16-bit PCM WAV.
const data = Buffer.alloc(LENGTH * 4);
for (let i = 0; i < LENGTH; i++) {
  data.writeInt16LE(Math.round(Math.tanh(mix[0][i] * scale) * 32767), i * 4);
  data.writeInt16LE(Math.round(Math.tanh(mix[1][i] * scale) * 32767), i * 4 + 2);
}
const header = Buffer.alloc(44);
header.write('RIFF', 0);
header.writeUInt32LE(36 + data.length, 4);
header.write('WAVEfmt ', 8);
header.writeUInt32LE(16, 16);
header.writeUInt16LE(1, 20);
header.writeUInt16LE(2, 22);
header.writeUInt32LE(SR, 24);
header.writeUInt32LE(SR * 4, 28);
header.writeUInt16LE(4, 32);
header.writeUInt16LE(16, 34);
header.write('data', 36);
header.writeUInt32LE(data.length, 40);
mkdirSync(new URL('../public/', import.meta.url), { recursive: true });
writeFileSync(new URL('../public/promo-audio.wav', import.meta.url), Buffer.concat([header, data]));
console.log(`Wrote public/promo-audio.wav (${DURATION}s, peak normalised from ${peak.toFixed(2)})`);
