# P02: The Linux service and packages

- Status: In review
- Pull request: [#74](https://github.com/lucaswewa/microscope/pull/74)
- ADRs: none
- Spec: [phases.md#p02](../phases.md#p02)

## Summary

A Linux machine, the Pi included, runs a microscope on its own: as a systemd service from a `.deb`, or with the launcher from an archive, as on Windows.

- **The launcher on Linux** keeps its folder in XDG's data folder, stops its server on Ctrl-C, SIGTERM and SIGHUP, and can't leave the server running behind it.
- **The `.deb`'s service** starts at boot, restarts when the app asks or after a crash, and listens on this machine alone until P14 adds tokens.
- **The release workflow** builds a Linux archive and a `.deb` for x86_64 and ARM64, beside the Windows zip, and tries each before publishing.

## What was built

| Where | What |
|---|---|
| `crates/microscope-server/src/cli.rs` | `--service`: as `--managed`, but standard input doesn't stop the server. Once a managed server's standard input closes, it ends within `STOP_DEADLINE` (30 s), even if its shutdown hangs |
| `crates/microscope-launcher/src/setup.rs` | `default_root`: `%LOCALAPPDATA%\Microscope` on Windows; elsewhere `$XDG_DATA_HOME/microscope`, or `~/.local/share/microscope` |
| `crates/microscope-launcher/src/lib.rs` | On Linux, SIGTERM and SIGHUP (the terminal closing) stop the launcher as Ctrl-C does |
| `crates/microscope-launcher/src/supervise.rs` | On Linux, the server in its own process group, so Ctrl-C reaches the launcher alone, as on Windows |
| `release/linux/microscope.service` | The systemd unit |
| `release/linux/default` | `/etc/default/microscope`: the service's address and port |
| `release/linux/control`, `postinst`, `prerm`, `postrm` | The `.deb`'s control file, and its scripts: the `microscope` user, and the service enabled, started, stopped and disabled |
| `release/package.sh` | The Linux archive and the `.deb`, as `package.ps1` makes the zip |
| `.github/workflows/release.yml` | Linux jobs for x86_64 and ARM64, each with two smoke tests, and publishing as a job of its own, after all three |
| `crates/microscope-launcher/tests/launcher.rs` | The integration tests on Linux too, and SIGTERM's |
| `web/tests/e2e/launcher.spec.ts` | Restart through the launcher, on Linux too |

### The service

| | |
|---|---|
| Runs | `microscope-server --service -c /etc/microscope/config.json --host $MICROSCOPE_HOST --port $MICROSCOPE_PORT`, as the `microscope` user, also in the `dialout` and `video` groups, for serial stages and cameras |
| Address | `127.0.0.1:5000`, from `/etc/default/microscope` |
| Configuration | `/etc/microscope/config.json`: the simulated microscope's, with absolute folders |
| Settings and data | `/var/lib/microscope/settings`, `/var/lib/microscope/data` |
| Logs | `/var/log/microscope`, and the journal |

ADR-0033's rules, as systemd's:

| Exit | The launcher | The service |
|---|---|---|
| 0, a shutdown | Ends | Stays stopped until started again, or the next boot |
| 75, a restart | Starts the server again at once | Starts it again after 1 s |
| 2 or 3 | Ends | Stays stopped (`RestartPreventExitStatus`) |
| A crash | Starts it again after 1 s, doubling | Starts it again after 1 s |
| The sixth crash in a minute | Ends | Stays stopped (`StartLimitBurst`) |

## How to try it

On a Linux machine, x86_64 or ARM64, such as the Pi 5 with Raspberry Pi OS (64-bit). The packages are in the pull request's "Release" run, as the `linux-x86_64` and `linux-arm64` artifacts.

1. **The service:**

   ```sh
   sudo apt install ./microscope_0~dev_arm64.deb
   systemctl status microscope      # active (running)
   curl http://127.0.0.1:5000/api/v1/health
   ```

   - **From another computer:** set `MICROSCOPE_HOST=0.0.0.0` in `/etc/default/microscope`, run `sudo systemctl restart microscope`, and open `http://<the Pi's name>:5000/` there.
   - **In the app:** About says "Managed by the launcher: Yes: it can restart". Power's Restart brings the server back, as a new process (`systemctl show -p MainPID microscope`). Shut down stops it; `sudo systemctl start microscope` starts it again.
   - **After a reboot,** it's running again.
   - **Its files:** captures in `/var/lib/microscope/data`, logs in `/var/log/microscope` and `journalctl -u microscope`.
   - **Removing it:** `sudo apt remove microscope` stops it; `/var/lib/microscope` stays.
2. **The launcher,** from the archive:

   ```sh
   tar -xzf microscope-dev-linux-arm64.tar.gz
   ./microscope-dev-linux-arm64/microscope-launcher
   ```

   It opens the app in a browser, if there's a desktop, and says where the microscope is. Its files are in `~/.local/share/microscope`. Ctrl+C, or closing the terminal, stops it and its server.

## Hardware

None: the phase has no hardware. It ran on GitHub's Ubuntu runners: 22.04 x86_64 and ARM64 for the packages and their smoke tests, and 24.04 for CI. This PC has no WSL, so Linux was tried only there. The owner's Pi 5 is to try "How to try it".

## Design notes and deviations from the plan

- **How the server ends with the launcher on Linux.** The spec suggested `PR_SET_PDEATHSIG` or a process group.
  - **Neither works without `unsafe` code or a new crate:** the launcher can only set the death signal in a `pre_exec` hook, which is `unsafe`, and the workspace forbids `unsafe` code (ADR-0005). A process group alone ends nothing.
  - **Instead, standard input:** however the launcher ends, even killed, the kernel closes the pipe to the server's standard input. The server then stops gracefully, as it does when the launcher stops it.
  - **And a deadline:** stopping the Things has no time limit, and with hardware a driver's close could hang. So a managed server ends 30 s after its standard input closes, however its shutdown goes. This is the Job Object's guarantee, on every platform, with a graceful stop first. On Windows, the Job Object stays.
  - **The process group** stays too: Ctrl-C in a terminal reaches the launcher alone, which stops the server itself, as `CREATE_NEW_PROCESS_GROUP` does on Windows.
- **`--service`:** `--managed` stops the server once standard input closes, and systemd gives a service `/dev/null`, which is closed from the start. Without either option, the app couldn't restart the server. So `--service` is `--managed` without standard input: the server offers Restart and exits 75 for it, prints the ready line (to the journal), and SIGTERM stops it (ADR-0039).
- **On this machine alone,** by default, as the owner chose: until P14's tokens, anyone who can reach the server can drive the microscope. P14 can change the default once there's a token.
- **The restart rules,** in systemd's terms (the table above). They're close to ADR-0033's, with two differences:
  - **no doubling:** systemd's growing delay (`RestartSteps`, systemd 254 on) counts every restart since the service started, including the app's;
  - **a requested restart waits 1 s,** like a crash's.
- **The launcher's folder** is one root, as on Windows: everything in `~/.local/share/microscope`, logs included, rather than spreading them over XDG's data, state and configuration folders. One place to find, back up or delete.
- **Built on Ubuntu 22.04,** whose glibc (2.35) is older than Debian 12's (2.36), and so than Raspberry Pi OS's. `package.sh` reads the newest glibc version the binaries need, and puts it in the `.deb`'s `Depends`.
- **The `.deb` is made by hand** with `dpkg-deb`, rather than with `cargo-deb` or debhelper: a dozen lines, and no new tool. Its scripts do what debhelper's would: the user (`useradd --system`), then `systemctl enable` and `restart`, and on removal `stop` and `disable`. It's compressed with xz, which every `dpkg` reads, rather than Ubuntu 22.04's default, zstd.
- **A purge keeps** `/var/lib/microscope` and the `microscope` user, so images are never deleted without asking.
- **Hardening:** `NoNewPrivileges`, `ProtectSystem=full`, `ProtectHome` and `PrivateTmp`. A camera's SDK that needs more can loosen them with a drop-in (P07–P09).
- **The `.deb`'s maintainer** is `lucaswewa <lucaswewa@users.noreply.github.com>`, GitHub's address for the account, rather than a personal one.
- **The app says "launcher"** when a service manages the server: About's "Managed by the launcher", and Power's "The launcher stops with it". Changing the wording changes the visual baselines, so it's left for P11, the Pi's installation phase.
- **Size:** 501 changed lines (449 added, 52 removed) of hand-written code, scripts and workflow, within M's 600.

## Tests

- **The server:**
  - `a_service_ignores_standard_input_and_restarts_with_75`: with standard input closed, a service still serves, says it's managed, and exits 75 for a restart;
  - `closed_input_stops_the_server_and_then_ends_it`: closed input asks the server to stop, then ends it after the deadline.
- **The launcher:**
  - `the_root_is_in_the_platforms_data_folder`: Windows' folder, XDG's, the home's when XDG's is unset or relative, and `.microscope` with neither;
  - its integration tests run on Linux too: restarts, crashes, the lock, the host, a server that can't start, and that the server doesn't outlive a killed launcher;
  - `sigterm_stops_the_launcher_and_its_server`, on Linux: SIGTERM ends the launcher with 0, and its server with it.
- **End to end:** Restart through the launcher, now on Linux x86_64 too.
- **The release workflow, on each Linux runner:**
  - the archive's launcher starts its server, which answers its health check, and SIGTERM stops the launcher with 0;
  - the `.deb` installs; its service answers; the app's restart brings it back as a new process; and removing the package stops it.
- **On Windows,** locally and in CI, everything as before.
- **Flaky simulator tests, not this phase's:** this phase changes nothing they run, yet two failed once each on Linux, and passed when run again:
  - **`smart_scan`'s `a_scan_that_fails_goes_back_to_the_start`,** on x86_64. Its scan's folder was gone: a scan that ends with no image removes its folder, so this scan failed at its first site, before the stage limit the test sets. The test doesn't show that error.
  - **`stack`'s `smart_stacks_on_a_tilted_sample_keep_the_sharpest_planes`,** on ARM64: the smart stack 0.4 mm from the centre didn't succeed.
  - **Since 14 October,** four simulator tests have failed once each in about sixty Rust test jobs, on Windows before P01 as well as on Linux; each is a different test. See the follow-ups.

## Follow-ups and known gaps

- **The Pi:** the owner's run of "How to try it" on the Pi 5.
- **P11:** the installation guide for the Pi, and the app's wording for a server that a service manages.
- **P14:** listening on the network by default, once there's a token.
- **The AppImage** is P19's, with the desktop app.
- **The flaky simulator tests:** a small pull request of its own, to have them show why they fail (the scan's error, the stack's attempts), and then to fix the causes.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Protocols are written from their documentation, in our own words.
