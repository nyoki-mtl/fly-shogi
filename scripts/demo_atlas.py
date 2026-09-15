"""Build a display sample from MaleCNS soma coordinates, in CSR index order."""
import json
import struct

import numpy as np
import pyarrow.feather as feather


def build_atlas(data_root, graph, model):
    source = data_root / "data/raw/malecns-v1.0/body-annotations-male-cns-v1.0-minconf-0.5.feather"
    rows = feather.read_table(source).to_pylist()
    rows = sorted((r for r in rows if r["superclass"] and r["status"] != "Glia"), key=lambda r: r["bodyId"])
    with graph.open("rb") as stream:
        magic, n, _ = struct.unpack("<8sQQ", stream.read(24))
        ids = np.frombuffer(stream.read(n * 8), dtype="<u8")
    if magic != b"FLYCSR01" or [r["bodyId"] for r in rows] != ids.tolist():
        raise ValueError("Atlas annotations do not match graph IDs")
    circuit = json.loads(model.read_text(encoding="utf-8"))
    kc, mbon = set(circuit["kc"]), set(circuit["mbon"])
    # Soma coordinates are biological data. No invented locations for missing somata.
    available = [i for i, r in enumerate(rows) if r["somaLocation"] and not r["superclass"].startswith("vnc") and r["somaLocation"][2] < 65000]
    selected = set(available[::max(1, len(available) // 5500)]) | ((kc | mbon) & set(available))
    coords = np.array([rows[i]["somaLocation"] for i in available])
    low, high = coords.min(axis=0), coords.max(axis=0)
    center, scale = (low + high) / 2, (high - low).max() / 2
    nodes = []
    for i in sorted(selected):
        r = rows[i]
        p = ((np.array(r["somaLocation"]) - center) / scale).round(5).tolist()
        group = "mbon" if i in mbon else "kc" if i in kc else "other"
        nodes.append({"index": i, "id": str(r["bodyId"]), "p": p, "group": group})
    return {"nodes": nodes, "total_neurons": n, "source": source.name,
            "geometry": "MaleCNS v1.0 somaLocation; brain sample; missing somata omitted",
            "attribution": "FlyEM / MaleCNS v1.0 · CC BY 4.0"}
