"""What FreeCAD runs when make.py starts it: builds the models and the manufacturing outputs, and
writes the result as JSON.

It always ends the process, so FreeCAD never waits for a person, even after an error. The result
appears all at once, so make.py can stop FreeCAD if it hangs on its way out.
"""

import json
import os
import sys
import traceback

result = {"error": None}
try:
    sys.path.insert(0, os.environ["OPTIMA_SRC"])
    from optima_cad import build

    result = build.run(os.environ["OPTIMA_OUT"], os.environ["OPTIMA_OUTPUTS"])
except BaseException:
    result["error"] = traceback.format_exc()
finally:
    try:
        path = os.environ["OPTIMA_RESULT"]
        with open(path + ".part", "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2)
        os.replace(path + ".part", path)
    finally:
        os._exit(0)
