import { useThing } from '@/api/things'
import { ApiError } from '@/api/wot/errors'
import type { Point } from '@/live/contain'
import { useToast } from '@/ui/overlays'

/**
 * Double-click to move: brings a point of the live image, given as shares
 * of its width and height from its centre, to the centre, through the
 * camera–stage mapping. The point is scaled to the frames the mapping was
 * calibrated with, whatever size the stream is now. While a move runs,
 * further double-clicks are ignored: the move lock.
 */
export function useClickToMove() {
  const mapping = useThing('camera_stage_mapping')
  const toast = useToast()
  let moving = false

  return async (point: Point) => {
    if (!mapping.value || moving) return
    moving = true
    try {
      const [width, height] = (await mapping.value.read('image_resolution')) ?? []
      if (width === undefined || height === undefined) {
        toast.info('Calibrate the camera–stage mapping first, to move by double-clicking.')
        return
      }
      const move = { x: point.x * width, y: point.y * height }
      await mapping.value.run('move_in_image_coordinates', move)
    } catch (error) {
      toast.error("Couldn't move there.", {
        details: error instanceof ApiError ? (error.body ?? error.message) : error,
      })
    } finally {
      moving = false
    }
  }
}
