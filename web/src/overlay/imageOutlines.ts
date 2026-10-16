// The Image outlines layer (P53): where each image of a scan lies in its
// stitched mosaic or live preview, from the layout stitching records
// (`stitched/tiles.json`, or `smart_scan`'s `latest_preview_tiles.json`).
import type { OverlayLayer } from './types'

/** Where each image of a mosaic went, as stitching records it, in the mosaic's pixels. */
export interface TileLayout {
  size: [number, number]
  tiles: {
    /** The image's file name, in the scan's `images/`. */
    image: string
    /** Its number in the scan: 1 for the first taken. */
    number: number
    x: number
    y: number
    width: number
    height: number
  }[]
}

/** A dashed rectangle around each image, labelled with its number, its file name on hover. */
export function imageOutlines(layout: TileLayout): OverlayLayer {
  return {
    id: 'image-outlines',
    label: 'Image outlines',
    size: layout.size,
    shapes: layout.tiles.map(({ image, number, x, y, width, height }) => ({
      kind: 'rect',
      x,
      y,
      width,
      height,
      dashed: true,
      label: String(number),
      title: `${number}: ${image}`,
    })),
  }
}
