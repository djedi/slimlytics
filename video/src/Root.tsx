import { Composition, Still } from 'remotion';
import { OgImage } from './OgImage';
import { Poster } from './Poster';
import { Promo, PROMO_FRAMES } from './Promo';
import { FPS } from './theme';

export function Root() {
  return (
    <>
      <Composition id="Promo" component={Promo} durationInFrames={PROMO_FRAMES} fps={FPS} width={1920} height={1080} />
      <Still id="Poster" component={Poster} width={1920} height={1080} />
      <Still id="OgImage" component={OgImage} width={1200} height={630} />
    </>
  );
}
