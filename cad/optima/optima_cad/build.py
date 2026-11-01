"""Builds every model into a folder, with its parameters, facts and checks as sheets."""

import os

import FreeCAD as App

from . import assembly, cameras, document, drawings, head, inserts, layout, parts, stages, stand
from . import params as P

# Each model, and the prefixes of the parameters it's built from. The cameras' checks also use
# the optics in front of them.
OPTICS = ("SENSOR_", "C_MOUNT_", "OBJECTIVES", "TUBE_", "INFINITY_SPACE")
HEAD = ("HEAD_", "DECK_", "LOWER_DECK", "UPPER_DECK", "CHANGER_PLATE", "CHANGER_RAIL", "CHANGER_NUT",
        "CHANGER_SCREW", "DETENT", "SLIDER_", "FOLD_MOUNT", "PORT_BRACKET", "CAMERA_FOCUS", "EPI_PLATE",
        "EPI_TUBE", "SWITCH")
STAND = ("STAND_", "ALUMINIUM", "INSERT", "MICROPLATE", "DISH")
WHOLE = ("MASSES", "EXTRUSION_KG_PER_M", "ENCLOSURE", "INTERLOCK", "INTAKE", "FAN", "BAFFLE",
         "TYPICAL_W", "EPI_PLUG", "TRAY", "CARRIERS", "ELECTRONICS_KG", "DUCTS")
MODELS = [
    ("optical-layout", layout.build, None),
    ("xy-stage", stages.build_xy, ("XY_",)),
    ("z-lift", stages.build_lift, ("LIFT_",)),
    ("mono-camera", cameras.build_mono, OPTICS + ("MONO_",)),
    ("colour-camera", cameras.build_colour, OPTICS + ("COLOUR_",)),
    ("overview-camera", cameras.build_overview, ("OVERVIEW_",)),
    ("objectives", parts.build_objectives, ("OBJECTIVE", "PARFOCAL", "CHANGER_PITCH")),
    ("imaging-optics", parts.build_imaging,
     OPTICS + ("PARFOCAL", "PUPIL_DEPTH", "CHANGER_THICKNESS", "FILTER_", "FOLD_AFTER_LENS", "FLIP_",
              "CAGE_60", "MIRROR_")),
    ("epi-optics", parts.build_epi,
     ("LED_", "SM1", "CAGE_30", "CAGE_PLATE", "DICHROIC", "FILTER_ROUND", "SMALL_OPTIC")),
    ("motion-parts", parts.build_motion,
     ("MGN12", "TRAVEL_MARGIN", "NEMA11", "LEAD_SCREW", "CHANGER_PITCH", "FILTER_PITCH")),
    ("led-matrix", parts.build_matrix, ("MATRIX_", "OBJECTIVES")),
    ("electronics", parts.build_electronics, ("SKR_PICO", "PICO2", "LDD_H", "SUPPLY", "PEAK_W")),
    ("optical-head", head.build,
     HEAD + ("FILTER_", "MGN12", "TRAVEL_MARGIN", "NEMA11", "LEAD_SCREW", "CAGE_30", "CAGE_PLATE",
             "FLIP_")),
    ("stand", stand.build_stand, STAND + ("XY_", "LIFT_", "HEAD_PLATE")),
    ("sample-inserts", inserts.build_inserts, ("INSERT", "MICROPLATE", "DISH", "COVER_GLASS")),
    ("instrument", assembly.build_instrument,
     WHOLE + ("ALUMINIUM", "XY_MASS", "LIFT_MASS", "LIFT_TRAVEL", "MONO_MASS", "STAND_FOCUS",
              "STAND_BASE")),
]
# The parameters only the bought parts' and the head's own models use.
PARTS = ("XY_", "LIFT_", "OVERVIEW_", "CAGE_", "DICHROIC_HOLDER", "FLIP_", "LED_HOUSING", "SM1",
         "FILTER_ROUND", "MGN12", "TRAVEL_MARGIN", "NEMA11", "LEAD_SCREW", "SKR_PICO", "PICO2",
         "LDD_H", "SUPPLY", "PEAK_W") + HEAD + STAND + WHOLE


def uses(name, prefixes):
    """Whether a model is built from a parameter. The layout uses all but the bought parts'."""
    if prefixes is None:
        return not name.startswith(PARTS)
    return name.startswith(prefixes)


def run(out_dir, outputs_dir):
    """Builds every model into `out_dir`, and the manufacturing outputs into `outputs_dir`."""
    App.ParamGet("User parameter:BaseApp/Preferences/Document").SetBool("SaveThumbnail", False)
    result = {"freecad": ".".join(App.Version()[:3]), "models": [], "error": None}
    for name, build, prefixes in MODELS:
        doc = document.new(name)
        checks, facts = build(doc)
        params = [(k, repr(v)) for k, v in vars(P).items() if k.isupper() and uses(k, prefixes)]
        document.sheet(doc, "Parameters", [("Name", "Value")] + params)
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
    checks, facts = drawings.write(outputs_dir)
    result["models"].append({"name": "manufacturing-outputs", "facts": [[k, v] for k, v in facts],
                             "checks": [c._asdict() for c in checks]})
    return result
