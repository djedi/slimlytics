// Scene order and lengths, shared by the video (Promo.tsx) and the soundtrack
// (scripts/compose-audio.mjs) so sound effects stay in sync with the visuals.
// Plain TypeScript without imports, so Node can load it directly.

export const FPS = 30;
export const TRANSITION = 15;

export const SCENES = [
  { id: 'intro', frames: 105 },
  { id: 'privacy', frames: 135 },
  { id: 'spy', frames: 165 },
  { id: 'dashboard', frames: 165 },
  { id: 'agents', frames: 165 },
  { id: 'deploy', frames: 135 },
  { id: 'cta', frames: 120 }
] as const;

export type SceneId = (typeof SCENES)[number]['id'];

/** First frame of each scene on the overall timeline (transitions overlap the previous scene). */
export function sceneStart(id: SceneId): number {
  let start = 0;
  for (const scene of SCENES) {
    if (scene.id === id) return start;
    start += scene.frames - TRANSITION;
  }
  throw new Error(`Unknown scene ${id}`);
}

export const PROMO_FRAMES = SCENES.reduce((sum, scene) => sum + scene.frames, 0) - (SCENES.length - 1) * TRANSITION;
