"""Builds every model into a folder, with its parameters, facts and checks as sheets."""

import os

import FreeCAD as App

from . import document, layout
from . import params as P

MODELS = [("optical-layout", layout.build)]


def run(out_dir):
    App.ParamGet("User parameter:BaseApp/Preferences/Document").SetBool("SaveThumbnail", False)
    result = {"freecad": ".".join(App.Version()[:3]), "models": [], "error": None}
    params = [("Name", "Value")] + [(k, repr(v)) for k, v in vars(P).items() if k.isupper()]
    for name, build in MODELS:
        doc = document.new(name)
        checks, facts = build(doc)
        document.sheet(doc, "Parameters", params)
        document.sheet(doc, "Facts", [("Fact", "Value")] + facts)
        rows = [(c.name, "pass" if c.ok else "FAIL", c.detail) for c in checks]
        document.sheet(doc, "Checks", [("Check", "Result", "Detail")] + rows)
        document.save(doc, os.path.join(out_dir, name + ".FCStd"), name)
        App.closeDocument(doc.Name)
        result["models"].append({
            "name": name,
            "facts": [[k, v] for k, v in facts],
            "checks": [c._asdict() for c in checks],
        })
    return result
