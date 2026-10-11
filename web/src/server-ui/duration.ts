/** A number of seconds, as whole hours and minutes, and the seconds left. */
export interface Duration {
  hours: number
  minutes: number
  seconds: number
}

/** `total` seconds as hours, minutes and seconds. */
export function splitDuration(total: number): Duration {
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total - hours * 3600) / 60)
  // Rounded, so that 0.1 + 0.2 seconds doesn't show as 0.30000000000000004.
  const seconds = Math.round((total - hours * 3600 - minutes * 60) * 1e6) / 1e6
  return { hours, minutes, seconds }
}

/** The number of seconds in `duration`. */
export function joinDuration({ hours, minutes, seconds }: Duration): number {
  return hours * 3600 + minutes * 60 + seconds
}
