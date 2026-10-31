"""What FreeCAD runs when make.py starts it: builds the models and writes the result as JSON.

It always ends the process, so FreeCAD never waits for a person, even after an error.
"""

import json
import os
import sys
import traceback

result = {"error": None}
try:
    sys.path.insert(0, os.environ["OPTIMA_SRC"])
    from optima_cad import build

    result = build.run(os.environ["OPTIMA_OUT"])
except BaseException:
    result["error"] = traceback.format_exc()
finally:
    try:
        with open(os.environ["OPTIMA_RESULT"], "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2)
    finally:
        os._exit(0)
