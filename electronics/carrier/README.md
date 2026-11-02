# The OPTIMA carrier board

The board under the Pico 2. [`optima_carrier_board.md`](../../docs/misc/optima_carrier_board.md) describes it, [`kicad/carrier.pdf`](kicad/carrier.pdf) is its schematic, and [`kicad/fab/`](kicad/fab/) has what a maker needs.

## Changing it

Don't edit the schematic or the board in KiCad: the build replaces them.

1. Change [`circuit.py`](circuit.py) or [`placement.py`](placement.py).
2. Run the build, with KiCad 10 installed:

   ```sh
   python electronics/carrier/make.py
   ```

   It takes about 15 seconds, and prints every check.
3. If you moved a part, or changed a net, route it again (below). The checks fail until you do.
4. Commit the scripts with the rebuilt `kicad/`, `routes.ses` and `REPORT.md`.

`python electronics/carrier/make.py --check` builds into a temporary folder instead. It fails if a check fails, or if `kicad/` or `REPORT.md` isn't what the scripts build. Run it before every pull request. It doesn't compare the PDFs, the Gerbers' zip or the picture, which the rest stand for, or the fill of the pour, which KiCad changes by a few points from run to run.

## Routing it

The build doesn't need a router: it takes the tracks from `routes.ses`. Only a new route needs FreeRouting, which is a tool here, not part of this project:

1. Download `freerouting-2.5.0.jar` from [its release](https://github.com/freerouting/freerouting/releases/tag/v2.5.0), and a Java 25 runtime, such as [Temurin's](https://adoptium.net/temurin/releases/?version=25). Neither needs installing.
2. Run this, leaving out `JAVA` if `java` on your path is Java 25:

   ```sh
   FREEROUTING=path/to/freerouting-2.5.0.jar JAVA=path/to/java python electronics/carrier/make.py --route
   ```

It takes about a minute, and prints how many connections are open after each job. A new route gives a different board each time, so run it again if it stops short of 0, or if a check fails. Then read the copper: the DRC doesn't know about current.

What took some finding, in case FreeRouting changes:

- **The jar, not the command-line build.** Version 2.5.0's `freerouting-cli` crashes on its first board and then hangs ([#957](https://github.com/freerouting/freerouting/issues/957), [#1019](https://github.com/freerouting/freerouting/issues/1019)).
- **A chain of one-pass jobs.** A job's first pass does the work, and its later passes tear up about the GND net's worth of it. So each job starts from the last one's tracks.
- **GND as a net, not a plane.** As a plane, the router counts it as joined while its own tracks cut the pour into islands.
- **Cheaper vias.** At its own cost, it leaves crossings open sooner than use one.
- **Separate processes.** KiCad reads the project's net classes only when it loads a saved board in a new process, or it exports every net in one class. So `board.py` runs in steps.
- **Its options,** in `make.py`. Automatic narrowing and fan-out make tracks thinner than their class. Usage data goes out unless it's turned off, which `make.py` does.

FreeRouting leaves its settings and logs in your user folder: `AppData\Roaming\freerouting` and `AppData\Local\freerouting` on Windows.

## Files

| File | What it does |
|---|---|
| `circuit.py` | The parts and the nets that join their pins |
| `schematic.py` | Draws the circuit as a KiCad schematic, with this board's symbols, and writes the project, with its rules and net classes |
| `footprints.py` | The two footprints KiCad doesn't have |
| `placement.py` | Where each part goes, and the tracks laid out by hand |
| `board.py` | Builds the board in KiCad's Python: the parts, the fixed tracks, the routes and the pour |
| `routes.ses` | The router's tracks. FreeRouting writes this. |
| `checks.py` | Checks the netlist against the circuit, the interlock, the pins, levels and connectors, and the board |
| `make.py` | Runs all that, and KiCad's command line for its checks and the fabrication files |
| `kicad/` | The built KiCad project, netlist and PDF, with the fabrication files in `kicad/fab/` |
| `REPORT.md` | The checks' results |

The symbols and those two footprints are this project's own. The other footprints are named from KiCad's libraries, which allow it.
