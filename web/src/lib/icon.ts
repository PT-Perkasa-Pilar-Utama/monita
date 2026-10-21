// A picked picture becomes a 128 px square WebP (PNG where the browser cannot write WebP) as a
// data URL. The server stores at most 32 KB and never resizes (src/platform/image.rs).

export const ICON_PX = 128;
export const ICON_MAX = 32 * 1024;

/** Bytes a base64 data URL decodes to, near enough to compare with the cap. */
export function dataUrlBytes(url: string): number {
  const b64 = url.slice(url.indexOf(",") + 1);
  return Math.floor((b64.length * 3) / 4);
}

/** The centred square of a picture, scaled to ICON_PX, at the first quality under the cap. */
export async function shrinkIcon(file: File): Promise<string> {
  const picture = await createImageBitmap(file);
  const side = Math.min(picture.width, picture.height);
  const canvas = document.createElement("canvas");
  canvas.width = ICON_PX;
  canvas.height = ICON_PX;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("no canvas");
  ctx.drawImage(picture, (picture.width - side) / 2, (picture.height - side) / 2, side, side, 0, 0, ICON_PX, ICON_PX);
  for (const quality of [0.85, 0.6, 0.4]) {
    const url = canvas.toDataURL("image/webp", quality);
    if (dataUrlBytes(url) <= ICON_MAX) return url;
  }
  throw new Error("icon over the cap");
}
