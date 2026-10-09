/** What the app knows about itself and where it runs. */
export interface AppInfo {
  /** The app's name. */
  readonly name: string
  /** The web app's version, from `package.json`. */
  readonly version: string
  /** Where the app runs: a browser tab, or (later) the Tauri desktop app. */
  readonly kind: 'browser' | 'tauri'
}

/** The state of a local microscope service that the host manages. */
export type ServiceState = 'stopped' | 'starting' | 'running' | 'stopping' | 'failed'

/** Control of a local microscope service, for hosts that manage one. */
export interface ServiceControl {
  /** The service's state now. */
  state(): Promise<ServiceState>
  /** Starts the service, if it isn't running. */
  start(): Promise<void>
  /** Stops the service gracefully. */
  stop(): Promise<void>
  /** Stops the service and starts it again. */
  restart(): Promise<void>
}

/**
 * The services the app needs from wherever it runs (ADR-0010).
 *
 * The app uses these instead of browser or Tauri APIs, so the same app runs
 * in a browser tab now and in the Tauri desktop app later. Only the host
 * implementation differs.
 */
export interface HostAdapter {
  /** What the app knows about itself and where it runs. */
  readonly info: AppInfo
  /** Opens `url` outside the app: in a new browser tab, or the system browser. */
  openLink(url: string): void
  /** Saves `data` as a file called `name`: as a download, or through a save dialog. */
  saveFile(name: string, data: Blob): Promise<void>
  /**
   * Control of a local microscope service, if this host manages one. A
   * browser tab doesn't: the launcher manages the service (P45), and the
   * Tauri app will.
   */
  readonly service?: ServiceControl
}
