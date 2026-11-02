# The OPTIMA carrier board

The board under the Pico 2. [`optima_carrier_board.md`](../../docs/misc/optima_carrier_board.md) describes it, and [`kicad/carrier.pdf`](kicad/carrier.pdf) is its schematic.

## Changing it

Don't edit the schematic in KiCad: the build replaces it.

1. Change [`circuit.py`](circuit.py).
2. Run the build, with KiCad 10 installed:

   ```sh
   python electronics/carrier/make.py
   ```

   It takes a few seconds, and prints every check.
3. Commit the scripts with the rebuilt `kicad/` and `REPORT.md`.

`python electronics/carrier/make.py --check` builds into a temporary folder instead. It fails if a check fails, or if `kicad/` or `REPORT.md` isn't what the scripts build. Run it before every pull request. It doesn't compare the PDF, which holds its date; the schematic stands for it.

## Files

| File | What it does |
|---|---|
| `circuit.py` | The parts and the nets that join their pins |
| `schematic.py` | Draws the circuit as a KiCad schematic, with this board's symbols, and writes the project |
| `checks.py` | Checks the netlist against the circuit, the interlock, the pins, levels and connectors |
| `make.py` | Runs KiCad's command line for the electrical rules check, the netlist and the PDF, then the checks |
| `kicad/` | The built KiCad project, netlist and PDF |
| `REPORT.md` | The checks' results |

The symbols are this project's own. Footprints are named from KiCad's libraries, which allow it; E05 adds the two KiCad doesn't have.
