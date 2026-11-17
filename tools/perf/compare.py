#!/usr/bin/env python3
"""Alternate headless benchmark processes; store raw samples and robust summaries."""
import argparse
import hashlib
import json
import platform
import statistics
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKLOADS = [
    "web/public/demo/beacon-trail.ws",
    "fixtures/generated/sram128-2003.ws",
    "fixtures/generated/sound-hyper.ws",
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", action="append", required=True, metavar="LABEL=EXECUTABLE")
    parser.add_argument("--libretro-host", type=Path)
    parser.add_argument("--beetle", type=Path)
    parser.add_argument("--oswan", type=Path)
    parser.add_argument("--frames", type=int, default=3000)
    parser.add_argument("--warmup", type=int, default=120)
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.frames <= 0 or args.repeats <= 0 or args.warmup < 0:
        parser.error("frames/repeats must be positive and warmup nonnegative")
    if bool(args.beetle) != bool(args.libretro_host):
        parser.error("--beetle and --libretro-host must be supplied together")
    runners = []
    for value in args.native:
        label, executable = value.split("=", 1)
        runners.append((label, [str(Path(executable).resolve())], ["crystal"], True))
    if args.beetle:
        runners.append(("beetle", [str(args.libretro_host.resolve()), str(args.beetle.resolve())], [], False))
    if args.oswan:
        runners.append(("oswan", [str(args.oswan.resolve())], [], False))
    if len({r[0] for r in runners}) != len(runners):
        parser.error("runner labels must be unique")
    rows = []
    excluded = []
    diagnostics = []
    for rom in WORKLOADS:
        eligible = [r for r in runners if not (r[0] == "oswan" and "hyper" in rom)]
        verified = []
        for runner in eligible:
            label, prefix, suffix, native = runner
            command = prefix + [rom, str(args.warmup), "2"] + suffix
            row = json.loads(subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=120, check=True).stdout)
            diagnostics.append(dict(stage=label, rom=rom, result=row))
            markers = row.get("markers")
            expected = [3, 8, 0] if "beacon-trail" in rom else [0x600d, 1] if "hyper" in rom else [1]
            if markers is not None and markers[:len(expected)] != expected:
                if native:
                    raise RuntimeError(f"native workload failed: {label}/{rom}: {markers}")
                excluded.append(dict(stage=label, rom=rom, reason="cartridge success markers did not match", markers=markers))
                continue
            if not native and markers is None:
                raise RuntimeError("reference host must report cartridge success markers")
            verified.append(runner)
        eligible = verified
        for trial in range(args.repeats):
            # Alternate the order to reduce systematic warmup/thermal bias.
            for label, prefix, suffix, native in eligible[trial % len(eligible):] + eligible[:trial % len(eligible)]:
                command = prefix + [rom, str(args.warmup), str(args.frames)] + suffix
                process = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=120, check=True)
                row = json.loads(process.stdout)
                if row["frames"] < args.frames or row["sample_values"] <= 0:
                    raise RuntimeError(f"missing frame/audio work: {label}/{rom}")
                row.update(stage=label, rom=rom, trial=trial, native=native)
                rows.append(row)
        native_rows = [r for r in rows if r["rom"] == rom and r["native"]]
        for key in ["video_hash", "audio_hash", "ram_hash", "sample_values", "frames"]:
            if len({r[key] for r in native_rows}) != 1:
                raise RuntimeError(f"native output changed: {rom}/{key}")
    summary = []
    for rom in WORKLOADS:
        for label, *_ in runners:
            values = [r["us_per_frame"] for r in rows if r["stage"] == label and r["rom"] == rom]
            if not values:
                continue
            entry = dict(stage=label, rom=rom, median_us=statistics.median(values), min_us=min(values), max_us=max(values))
            summary.append(entry)
            print(f"{label:12} {Path(rom).name:24} {entry['median_us']:9.3f} us/frame [{min(values):.3f}, {max(values):.3f}]", flush=True)
    result = {
        "host": dict(system=platform.system(), release=platform.release(), machine=platform.machine()),
        "frames_requested": args.frames, "warmup": args.warmup, "repeats": args.repeats,
        "rom_sha256": {rom: hashlib.sha256((ROOT / rom).read_bytes()).hexdigest() for rom in WORKLOADS},
        "summary": summary, "samples": rows,
        "diagnostics": diagnostics, "excluded": excluded,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
