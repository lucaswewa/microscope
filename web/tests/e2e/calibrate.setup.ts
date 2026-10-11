import { expect, test as setup, type APIRequestContext } from '@playwright/test'

/** Runs an action through the API, and waits for it to end. */
async function run(request: APIRequestContext, path: string, data: object = {}) {
  let invocation = await (await request.post(`/api/v1/${path}`, { data })).json()
  while (['pending', 'running'].includes(invocation.status)) {
    await new Promise((resolve) => setTimeout(resolve, 200))
    invocation = await (await request.get(invocation.href)).json()
  }
  expect(invocation.status).toBe('completed')
}

// The tests expect a calibrated microscope, or the calibration wizard would
// open over them (P29). The z direction is confirmed as it is.
setup('calibrate the microscope', async ({ request }) => {
  setup.setTimeout(120_000)
  const read = async (path: string) => (await request.get(`/api/v1/${path}`)).json()
  if (await read('stage/calibration_required')) {
    const inverted = await read('stage/axis_inverted')
    await run(request, 'stage/calibrate_z_direction', { z_inverted: inverted.z })
  }
  if (await read('camera/calibration_required')) await run(request, 'camera/full_auto_calibrate')
  if (await read('camera_stage_mapping/calibration_required'))
    await run(request, 'camera_stage_mapping/calibrate_xy')
})
