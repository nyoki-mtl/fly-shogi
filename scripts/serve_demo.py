"""Serve the local demo and keep one full-MaleCNS Rust worker resident."""
import argparse
import json
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import threading
from demo_atlas import build_atlas
from demo_teacher import DLSuishoTeacher
from runtime import build_binary

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port",type=int,default=8765)
    parser.add_argument("--data-root",type=Path,default=ROOT,help="Directory containing prepared MaleCNS data and work_dir/malecns-csr")
    parser.add_argument("--model",type=Path,required=True,help="Trained fly circuit JSON")
    parser.add_argument("--teacher",type=Path,required=True,help="DL Suisho ONNX file, obtained separately")
    parser.add_argument("--temperature",type=float,default=1.0)
    args = parser.parse_args()
    args.model = args.model.resolve()
    args.data_root = args.data_root.resolve()
    teacher = DLSuishoTeacher(args.teacher, temperature=args.temperature)
    graph = args.data_root/"work_dir/malecns-csr/graph.bin"
    metadata = graph.with_name("metadata.json")
    if not graph.exists() or not metadata.exists():
        parser.error("Run uv run python scripts/prepare_graph.py first")
    if not args.model.exists():
        parser.error("Train a fly circuit first, or pass --model PATH to a pretrained checkpoint")
    atlas = build_atlas(args.data_root, graph, args.model)
    atlas_path = ROOT/"work_dir/demo-atlas.json"
    atlas_path.parent.mkdir(parents=True,exist_ok=True)
    atlas_path.write_text(json.dumps(atlas),encoding="utf-8")
    binary = build_binary("fly-shogi-demo")
    worker = subprocess.Popen([str(binary),str(graph),str(metadata),str(args.model),str(atlas_path)],
        stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,encoding="utf-8",bufsize=1)
    try:
        ready = json.loads(worker.stdout.readline())
        if not ready.get("ready"):
            raise RuntimeError("Native worker failed to initialize")
        lock = threading.Lock()

        class Handler(SimpleHTTPRequestHandler):
            def __init__(self,*a,**kw):
                super().__init__(*a,directory=str(ROOT/"web"),**kw)

            def do_POST(self):
                if self.path != "/api":
                    self.send_error(404)
                    return
                origin = self.headers.get("Origin")
                if origin and origin not in {f"http://127.0.0.1:{args.port}",f"http://localhost:{args.port}"}:
                    self.send_error(403)
                    return
                try:
                    length = int(self.headers.get("Content-Length","0"))
                    if not 0<length<=65536:
                        raise ValueError("Request too large or empty")
                    request = json.loads(self.rfile.read(length))
                    with lock:
                        worker.stdin.write(json.dumps(request,ensure_ascii=False)+"\n")
                        worker.stdin.flush()
                        line = worker.stdout.readline()
                        if not line:
                            raise RuntimeError("The worker stopped. Restart the server.")
                        response = json.loads(line)
                        if 'error' not in response:
                            if 'decision' in response:
                                response['feedback'] = teacher.feedback(response.pop('decision'), response['move'], response.pop('predicted_reward'))
                    status = 400 if "error" in response else 200
                except (ValueError,OSError,RuntimeError) as error:
                    response,status = {"error":str(error)},400
                raw = json.dumps(response,ensure_ascii=False).encode("utf-8")
                self.send_response(status)
                self.send_header("Content-Type","application/json; charset=utf-8")
                self.send_header("Cache-Control","no-store")
                self.send_header("Content-Length",str(len(raw)))
                self.end_headers()
                self.wfile.write(raw)

            def do_GET(self):
                if self.path == "/api/atlas":
                    raw = json.dumps(atlas).encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type","application/json")
                    self.send_header("Content-Length",str(len(raw)))
                    self.end_headers()
                    self.wfile.write(raw)
                else:
                    super().do_GET()

        with ThreadingHTTPServer(("127.0.0.1",args.port),Handler) as server:
            print(f"Fly Shogi demo: http://127.0.0.1:{args.port} — {ready['neurons']:,} neurons loaded",flush=True)
            try:
                server.serve_forever()
            except KeyboardInterrupt:
                pass
    finally:
        worker.terminate()
        worker.wait(timeout=10)


if __name__=="__main__":
    main()
