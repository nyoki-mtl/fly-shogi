"""Convert the pinned full MaleCNS graph to deterministic little-endian CSR."""
import argparse
import json
import struct
from pathlib import Path

import numpy as np
import pyarrow as pa
import pyarrow.feather as feather
import pyarrow.ipc as ipc

from runtime import ROOT, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data-root', type=Path, default=ROOT)
    args = parser.parse_args()
    lock = json.loads((ROOT / "config/data-sources.lock.json").read_text(encoding="utf-8"))
    paths = {}
    for key, spec in lock["files"].items():
        path = args.data_root / "data/raw/malecns-v1.0" / spec["url"].rsplit("/", 1)[-1]
        if path.stat().st_size != spec["bytes"] or digest(path) != spec["sha256"]:
            raise ValueError(f"Source checksum mismatch: {key}")
        paths[key] = path
    rows = feather.read_table(paths["annotations"]).to_pylist()
    rows = sorted((r for r in rows if r["superclass"] and r["status"] != "Glia"), key=lambda r: r["bodyId"])
    ids = np.array([r["bodyId"] for r in rows], dtype="<u8")
    if len(np.unique(ids)) != len(ids):
        raise ValueError("Duplicate bodyId")
    nt = feather.read_table(paths["neurotransmitters"])
    lookup = dict(zip(nt["body"].to_pylist(), nt["consensus_nt"].to_pylist(), strict=True))
    sign = np.array([-1 if lookup.get(int(i)) in {"gaba", "glutamate", "histamine"} else 1 for i in ids], dtype="<i4")
    pres, posts, contacts = [], [], []
    with pa.memory_map(str(paths["edges"]), "r") as source:
        reader = ipc.open_file(source)
        for b in range(reader.num_record_batches):
            batch = reader.get_batch(b)
            pre, post, weight = [batch.column(batch.schema.get_field_index(c)).to_numpy() for c in ("body_pre", "body_post", "weight")]
            i, j = np.searchsorted(ids, pre), np.searchsorted(ids, post)
            keep = (i < len(ids)) & (j < len(ids)) & (ids[np.minimum(i,len(ids)-1)] == pre) & (ids[np.minimum(j,len(ids)-1)] == post)
            w = weight[keep]
            if np.any(w <= 0) or np.any(w > np.iinfo(np.int32).max) or np.any(w != np.floor(w)):
                raise ValueError("Invalid contact count")
            pres.append(i[keep].astype("<u4")); posts.append(j[keep].astype("<u4")); contacts.append(w.astype("<i4"))
    pre, post, weight = map(np.concatenate, (pres, posts, contacts))
    del pres, posts, contacts
    order = np.lexsort((post, pre))
    pre, post, weight = pre[order], post[order], weight[order]
    if np.any((pre[1:] == pre[:-1]) & (post[1:] == post[:-1])):
        raise ValueError("Duplicate directed edge; define aggregation before proceeding")
    counts = np.bincount(pre, minlength=len(ids)).astype("<u8")
    offsets = np.concatenate((np.array([0],dtype="<u8"), np.cumsum(counts,dtype="<u8")))
    contact_total = int(weight.sum(dtype=np.int64))
    weight *= sign[pre]
    output = args.data_root / "work_dir/malecns-csr"
    output.mkdir(parents=True, exist_ok=True)
    graph = output / "graph.bin"
    with graph.open("wb") as f:
        f.write(b"FLYCSR01" + struct.pack("<QQ",len(ids),len(post)))
        for array in (ids, offsets, post, weight):
            f.write(array.tobytes())
    if (len(ids), len(post), contact_total) != (166700, 25582938, 124177617):
        raise ValueError("Pinned MaleCNS graph counts differ")
    if digest(graph) != '03a0ba851c614c5d14302fbfa12eda957299723ab290bbb71db83bb70eb3e127':
        raise ValueError("Pinned MaleCNS graph hash differs")
    outputs = {"descending_neuron", "descending_neuron_tbc", "cb_motor", "vnc_motor"}
    metadata = {
        "format": "FLYCSR01", "dataset": lock["dataset"], "sources": lock["files"],
        "neurons":len(ids),"edges":len(post),"contacts":contact_total,
        "graph_sha256":digest(graph), "graph_bytes":graph.stat().st_size,
        "sign_rule":"gaba/glutamate/histamine=-1; all others including unknown=+1",
        "negative_nodes":int((sign<0).sum()),
        "input_pool":[i for i,r in enumerate(rows) if r["superclass"]=="cb_sensory"],
        "outputs":[i for i,r in enumerate(rows) if r["superclass"] in outputs],
    }
    (output / "metadata.json").write_text(json.dumps(metadata,indent=2)+"\n",encoding="utf-8",newline="\n")
    print(json.dumps({k:v for k,v in metadata.items() if k not in {"input_pool","outputs","sources"}},indent=2))


if __name__ == "__main__":
    main()
