# Slimlytics promo video

The 30-second homepage tour, built with [Remotion](https://www.remotion.dev/) (React → MP4).

```sh
cd video
npm install
npm run studio    # live preview and scrubbing
npm run publish   # render, re-encode for the web, and write frontend/static/video/
```

- `src/scenes.tsx` — the seven scenes (hook, privacy, Spy, dashboard, AI agents, open source, call to action)
- `src/Promo.tsx` — scene order and lengths; the total must stay at 900 frames (30 s at 30 fps)
- `src/Poster.tsx` — the still shown before playback
- `src/theme.ts` — brand colours (shared with the marketing site) and fonts
- `src/timeline.ts` — scene lengths, shared with the soundtrack so effects stay in sync
- `scripts/compose-audio.mjs` — synthesises the music and sound effects (original audio, no
  licences) into `public/promo-audio.wav`; `npm run audio` regenerates it

`publish` needs `ffmpeg`. The web encode is H.264 with `+faststart` and AAC audio normalised to -16 LUFS; the
homepage loads it with `preload="none"`, so it costs nothing until a visitor presses play.
