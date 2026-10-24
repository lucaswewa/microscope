// The Corrections layer (P13): how far correlation moved each image of a
// stitched mosaic from where the stage put it, from the layout stitching
// records (`stitched/tiles.json`).
import type { TileLayout } from './imageOutlines'
import type { OverlayLayer, Shape } from './types'

/**
 * A line from where the stage put each image's middle to where it is, and how
 * far that is, in pixels. A layout that doesn't say where the stage put the
 * images, or where none moved, has none.
 */
export function corrections(layout: TileLayout): OverlayLayer | undefined {
  const moves = layout.tiles.map((tile) =>
    tile.stage_x === undefined || tile.stage_y === undefined
      ? undefined
      : { tile, dx: tile.x - tile.stage_x, dy: tile.y - tile.stage_y },
  )
  if (moves.some((move) => !move) || moves.every((move) => !move!.dx && !move!.dy)) return
  const shapes: Shape[] = moves.flatMap((move) => {
    const { tile, dx, dy } = move!
    const [x, y] = [tile.x + tile.width / 2, tile.y + tile.height / 2]
    const moved = Math.round(Math.hypot(dx, dy))
    const title = `${tile.number}: ${tile.image}, ${moved} px from where the stage put it`
    return [
      {
        kind: 'line',
        points: [
          [x - dx, y - dy],
          [x, y],
        ],
        title,
      },
      { kind: 'label', x, y, text: `${moved} px`, title },
    ]
  })
  return { id: 'corrections', label: 'Corrections', size: layout.size, shapes }
}
