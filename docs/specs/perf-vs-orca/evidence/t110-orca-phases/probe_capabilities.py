#!/usr/bin/env python3
"""t110 Phase 0: what per-stage introspection does this Orca install expose?

Automates the two channels that can be tested without a helper process:

  1. the pinned `--logfile` CLI log at --debug 4: does it carry timestamps, and does
     it flush per stage or per buffer? Measured by tapping the file every 2 ms and
     reporting, for each stage marker, the arrival time and how many markers share an
     arrival burst.
  2. ETW kernel CPU sampling: `wpr -start cpu -filemode` (and the xperf equivalent).
     Records the exact failure text when the host refuses.

The `--pipe` channel needs a named-pipe server and is recorded in FINDINGS.md with
its command and the server's NO_CONNECTION result; it is not automatable here.

Usage: python probe_capabilities.py --out capabilities.json
"""

import argparse
import json
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import run_campaign as rc  # noqa: E402

MARKERS = (
    "Starting the slicing process",
    "Slicing volumes",
    "Make overhang printable",
    "Generating walls",
    "Detecting solid surfaces",
    "Preparing fill surfaces",
    "Discovering vertical shells",
    "Processing external surfaces",
    "Bridge over infill - Start",
    "is_support_necessary",
    "Slicing process finished",
    "Will export G-code",
    "Exporting G-code",
    "Exporting G-code finished",
    "Slic3r::CLI::run, Finished",
)


def probe_log(log_path, outdir):
    """Run one benchy classic-off slice while tapping its pinned log."""
    cell = rc.CELLS[0]
    os.makedirs(outdir, exist_ok=True)
    if os.path.exists(log_path):
        os.remove(log_path)
    cmd = rc.build_cmd(cell, outdir, log_path)
    # --debug 4: the richer level, to give the log its best chance of carrying
    # per-layer detail. Level is substituted in place of the runner's --debug 3.
    idx = cmd.index("--debug")
    cmd[idx + 1] = "4"

    t0 = time.perf_counter_ns()
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    arrivals = []
    offset = 0
    pending = b""
    while True:
        size = os.path.getsize(log_path) if os.path.exists(log_path) else 0
        if size > offset:
            with open(log_path, "rb") as fh:
                fh.seek(offset)
                chunk = fh.read()
            offset += len(chunk)
            pending += chunk
            while b"\n" in pending:
                line, pending = pending.split(b"\n", 1)
                text = line.decode("utf-8", "replace").rstrip("\r")
                for m in MARKERS:
                    if text.startswith(m):
                        arrivals.append(
                            {
                                "t_ms": round((time.perf_counter_ns() - t0) / 1e6, 1),
                                "marker": m,
                            }
                        )
                        break
        if proc.poll() is not None:
            break
        time.sleep(0.002)
    wall_ms = (time.perf_counter_ns() - t0) / 1e6

    # Group arrivals into bursts: markers that land within 5 ms of each other.
    bursts = []
    for a in arrivals:
        if bursts and a["t_ms"] - bursts[-1][-1]["t_ms"] <= 5.0:
            bursts[-1].append(a)
        else:
            bursts.append([a])
    return {
        "wall_ms": round(wall_ms, 1),
        "exit_code": proc.returncode,
        "log_bytes": os.path.getsize(log_path) if os.path.exists(log_path) else 0,
        "marker_count": len(arrivals),
        "burst_count": len(bursts),
        "largest_burst": max((len(b) for b in bursts), default=0),
        "first_marker_ms": arrivals[0]["t_ms"] if arrivals else None,
        "last_marker_ms": arrivals[-1]["t_ms"] if arrivals else None,
        "stage_marker_span_ms": None
        if len(arrivals) < 2
        else round(arrivals[-1]["t_ms"] - arrivals[0]["t_ms"], 1),
        "markers": arrivals,
    }


def probe_etw():
    out = {"wpr": {}, "xperf": {}}
    r = subprocess.run(
        ["wpr", "-start", "cpu", "-filemode"],
        capture_output=True,
        text=True,
        timeout=120,
    )
    out["wpr"]["start_exit"] = r.returncode
    out["wpr"]["start_stdout"] = r.stdout.strip()
    out["wpr"]["start_stderr"] = r.stderr.strip()
    r2 = subprocess.run(["wpr", "-cancel"], capture_output=True, text=True, timeout=120)
    out["wpr"]["cancel_exit"] = r2.returncode
    out["wpr"]["cancel_stdout"] = r2.stdout.strip()

    xperf = (
        r"C:\Program Files (x86)\Windows Kits\10\Windows Performance Toolkit\xperf.exe"
    )
    if os.path.exists(xperf):
        r3 = subprocess.run(
            [xperf, "-on", "PROC_THREAD+LOADER+PROFILE", "-stackwalk", "PROFILE"],
            capture_output=True,
            text=True,
            timeout=120,
        )
        out["xperf"]["on_exit"] = r3.returncode
        out["xperf"]["on_output"] = (r3.stdout + r3.stderr).strip()
        r4 = subprocess.run(
            [xperf, "-stop"], capture_output=True, text=True, timeout=120
        )
        out["xperf"]["stop_output"] = (r4.stdout + r4.stderr).strip()
    else:
        out["xperf"]["on_output"] = "xperf.exe not present"
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--out",
        default=os.path.join(
            os.path.dirname(os.path.abspath(__file__)), "capabilities.json"
        ),
    )
    ap.add_argument(
        "--rawdir", default=os.path.join(rc.REPO, "target", "t110-capability-probe")
    )
    args = ap.parse_args()
    os.makedirs(args.rawdir, exist_ok=True)

    log_probe = probe_log(
        os.path.join(args.rawdir, "logtap.log"), os.path.join(args.rawdir, "out")
    )
    print(
        f"log: markers={log_probe['marker_count']} bursts={log_probe['burst_count']} "
        f"largest_burst={log_probe['largest_burst']} span={log_probe['stage_marker_span_ms']} ms "
        f"wall={log_probe['wall_ms']} ms log_bytes={log_probe['log_bytes']}"
    )
    etw = probe_etw()
    print("etw wpr start:", etw["wpr"]["start_exit"], etw["wpr"]["start_stdout"][:160])
    print("etw xperf:", etw["xperf"]["on_output"][:160])

    doc = {
        "note": (
            "t110 Phase 0 capability census. log_probe is one benchy-classic-off slice at "
            "--debug 4 with the log tapped every 2 ms; bursts are markers arriving within "
            "5 ms of each other. etw records the host's refusal text verbatim."
        ),
        "log_probe": log_probe,
        "etw": etw,
    }
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1)
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
