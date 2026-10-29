#!/usr/bin/env python3
"""Measure how long the base station's native screen stays up, from a phone video of the OLED.

Usage:
    python3 analyze_osd_video.py VIDEO.mp4 [GXOLED_TRACE.log] [--roi x0,y0,x1,y1]

Needs: ffmpeg + ffprobe on PATH, Python 3 with numpy. Nothing is uploaded or written outside
stdout (use --dump-features to also write the per-frame signal as CSV).

What it does
  1. Decodes the video (downscaled, grayscale, constant 60 fps timeline from the container's
     timestamps) and finds the OLED automatically: the part of the image whose brightness changes
     over time. `--roi` overrides this (coordinates in the *decoded, display-rotated* frame, i.e.
     what a player shows, at 1/4 scale of a 1080-wide portrait video: width 270).
  2. Classifies every frame as "native screen" or "GxOled content". The native home screen always
     lights the top-centre tab ("USB ..."); GxOled's clock and stamp never touch the top band.
  3. Reports every native-screen episode: start, end, duration.
  4. With the trace log of the same session (GXOLED_OSD_TRACE_FILE, stamp or lab mode) it aligns
     video time to log time and prints, per episode, the dial times, the experiment step (lab mode),
     the frame that ended the episode and the visible time measured from the first/last dial.

Alignment: the first video frame of each episode lags the first dial report of its burst by a
roughly constant amount (camera + display latency, 2.1-2.9 s in the videos so far). That constant is
found by searching for the shift that lines the long episodes up with the dial bursts of the log.
Every episode (also short flashes) is then matched to the dial reports that fall inside it, so
bursts the native screen merged, or turns whose screen was suppressed, do not confuse the pairing.
The ending frame is the last log FRAME written before the episode ended; the frame before it was
discarded by the device, which bounds how long the device's own lock lasted.
"""
import json
import re
import subprocess
import sys

import numpy as np

SCALE_W = 270  # decoded width (1080/4): plenty for a band-brightness signal


def probe(video):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
         "frame=pts_time", "-of", "csv=p=0", video],
        capture_output=True, text=True, check=True).stdout.split()
    return np.array([float(x.strip(",")) for x in out if x.strip(",")])


def decode(video):
    # Let ffmpeg apply the rotation metadata, then scale to a small gray frame.
    info = json.loads(subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
         "stream=width,height:stream_side_data=rotation", "-of", "json", video],
        capture_output=True, text=True, check=True).stdout)["streams"][0]
    w, h = info["width"], info["height"]
    rot = 0
    for sd in info.get("side_data_list", []):
        rot = int(sd.get("rotation", 0))
    if abs(rot) in (90, 270):
        w, h = h, w
    sh = int(round(SCALE_W * h / w / 2) * 2)
    cmd = ["ffmpeg", "-v", "error", "-i", video, "-vf", f"scale={SCALE_W}:{sh},format=gray",
           "-f", "rawvideo", "-"]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    size = SCALE_W * sh
    frames = []
    while True:
        buf = proc.stdout.read(size)
        if len(buf) < size:
            break
        frames.append(np.frombuffer(buf, dtype=np.uint8).reshape(sh, SCALE_W))
    return np.stack(frames)


def find_roi(frames):
    # Pixels whose brightness swings strongly over time belong to the OLED (the rest of the
    # scene is static apart from a hand). Take the box around the strongest 1 % of them.
    sample = frames[:: max(1, len(frames) // 600)].astype(np.float32)
    swing = sample.max(axis=0) - sample.min(axis=0)
    mask = swing > np.percentile(swing, 99)
    ys, xs = np.nonzero(mask)
    x0, x1 = np.percentile(xs, [2, 98]).astype(int)
    y0, y1 = np.percentile(ys, [2, 98]).astype(int)
    pad_x, pad_y = int(0.08 * (x1 - x0)), int(0.15 * (y1 - y0))
    return max(0, x0 - pad_x), max(0, y0 - pad_y), min(frames.shape[2], x1 + pad_x), min(frames.shape[1], y1 + pad_y)


def otsu(values):
    hist, edges = np.histogram(values, bins=128)
    centers = (edges[:-1] + edges[1:]) / 2
    weight1 = np.cumsum(hist)
    weight2 = np.cumsum(hist[::-1])[::-1]
    mean1 = np.cumsum(hist * centers) / np.maximum(weight1, 1)
    mean2 = (np.cumsum((hist * centers)[::-1]) / np.maximum(weight2[::-1], 1))[::-1]
    var = weight1[:-1] * weight2[1:] * (mean1[:-1] - mean2[1:]) ** 2
    return centers[:-1][np.argmax(var)]


def episodes(times, flags, min_len=1.0):
    out, start = [], None
    for i, flag in enumerate(flags):
        if flag and start is None:
            start = i
        if (not flag or i == len(flags) - 1) and start is not None:
            end = i if not flag else i + 1
            if times[min(end, len(times) - 1)] - times[start] >= min_len:
                out.append((times[start], times[min(end, len(times) - 1)]))
            start = None
    return out


def parse_log(path):
    dials, frames, groups = [], [], []
    for line in open(path, encoding="utf-8", errors="replace"):
        m = re.match(r"\[(\d+)ms pid=\d+\] (.*)", line)
        if not m:
            continue
        t, msg = int(m.group(1)) / 1000.0, m.group(2)
        if msg.startswith("DIAL "):
            dials.append(t)
        elif msg.startswith("FRAME seq="):
            seq = int(re.search(r"seq=(\d+)", msg).group(1))
            step = re.search(r"step=(\S+)", msg)
            frames.append((t, seq, step.group(1) if step else "-"))
        elif msg.startswith("LAB_GROUP"):
            groups.append((t, re.search(r"step=(\S+)", msg).group(1)))
    return dials, frames, groups


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        print(__doc__)
        return 2
    video = args[0]
    log = args[1] if len(args) > 1 else None
    roi = None
    dump = False
    for a in sys.argv[1:]:
        if a.startswith("--roi="):
            roi = tuple(int(v) for v in a[6:].split(","))
        if a == "--dump-features":
            dump = True

    times = probe(video)
    frames = decode(video)
    n = min(len(times), len(frames))
    times, frames = times[:n], frames[:n]
    x0, y0, x1, y1 = roi or find_roi(frames)
    print(f"video {video}: {n} frames, {times[-1]:.2f} s; OLED roi = {x0},{y0},{x1},{y1} (decoded {frames.shape[2]}x{frames.shape[1]})")

    crop = frames[:, y0:y1, x0:x1].astype(np.float32)
    band = crop[:, : max(2, int(0.28 * crop.shape[1])), :]
    feature = band.mean(axis=(1, 2))
    threshold = otsu(feature)
    flags = feature > threshold
    # Drop single-frame glitches (a finger passing over the OLED).
    for _ in range(2):
        flags = np.convolve(flags.astype(int), np.ones(5, int), "same") >= 3
    eps = episodes(times, flags)
    print(f"native-screen threshold {threshold:.1f}; {len(eps)} episode(s)")
    if dump:
        with open(video + ".features.csv", "w") as f:
            f.write("t,feature,native\n")
            for t, v, fl in zip(times, feature, flags):
                f.write(f"{t:.4f},{v:.2f},{int(fl)}\n")

    if not log:
        for i, (a, b) in enumerate(eps):
            print(f"  episode {i}: {a:7.2f} -> {b:7.2f} s  = {b - a:5.2f} s")
        return 0

    dials, log_frames, groups = parse_log(log)
    if not dials:
        print("the log has no DIAL lines")
        return 1
    short_eps = episodes(times, flags, min_len=0.15)
    long_eps = [e for e in short_eps if e[1] - e[0] >= 2.0]
    if not long_eps:
        print("no native-screen episode of 2 s or more to align with")
        return 1
    # Dial bursts: reports with no other report in the 3 s before them.
    bursts = [t for i, t in enumerate(dials) if i == 0 or t - dials[i - 1] >= 3.0]
    grid = np.arange(0.3, 8.0, 0.01)
    cost = [sum(min(abs(a - o - b) for b in bursts) for a, _ in long_eps) for o in grid]
    guess = float(grid[int(np.argmin(cost))])
    near = [a - min(bursts, key=lambda b: abs(a - guess - b)) for a, _ in long_eps]
    near = [v for v in near if abs(v - guess) < 0.4]
    offset = float(np.median(near)) if near else guess
    print(f"video-start minus first-dial (median of {len(near)} long episodes) = {offset:.3f} s; "
          f"spread {max(near) - min(near):.2f} s" if near else f"offset guess {offset:.2f} s")

    def step_at(t):
        label = "-"
        for gt, step in groups:
            if gt <= t + 0.01:
                label = step
        return label

    rows = []
    for a, b in short_eps:
        inside = [t for t in dials if a - offset - 0.6 <= t <= b - offset + 0.2]
        if not inside:
            continue
        first, last = inside[0], inside[-1]
        end_log = b - offset
        before = [(t, seq) for t, seq, _ in log_frames if last <= t < end_log - 0.02]
        after = [(t, seq) for t, seq, _ in log_frames if t >= end_log - 0.02]
        rows.append({
            "a": a, "b": b, "first": first, "last": last, "n": len(inside), "step": step_at(first),
            "dur": b - a, "vis_first": end_log - first, "vis_last": end_log - last,
            "ending": before[-1] if before else None, "next": after[0] if after else None,
            "discarded": before[:-1],
        })
    print()
    print(" #  | step                   | video s..s      | dials | OSD s | after first | after last | frame that ended it (s after last dial) | discarded before it")
    for i, r in enumerate(rows):
        end_txt = f"seq {r['ending'][1]} @ {r['ending'][0] - r['last']:.2f}" if r["ending"] else "n/a"
        disc = ", ".join(f"{t - r['last']:.2f}" for t, _ in r["discarded"]) or "-"
        flash = "  (flash)" if r["dur"] < 1.0 else ""
        print(f"{i:3d} | {r['step']:22s} | {r['a']:6.2f}..{r['b']:6.2f} | {r['n']:5d} | {r['dur']:5.2f} | {r['vis_first']:11.2f} | {r['vis_last']:10.2f} | {end_txt:39s} | {disc}{flash}")

    print()
    print("per step (episodes of 1 s or more; 'flashes' are native screens shorter than 1 s)")
    print("step                   | n | visible after last dial: min / median / max s | flashes")
    order = []
    for r in rows:
        if r["step"] not in order:
            order.append(r["step"])
    for step in order:
        mine = [r for r in rows if r["step"] == step]
        longs = [r["vis_last"] for r in mine if r["dur"] >= 1.0]
        flashes = sum(1 for r in mine if r["dur"] < 1.0)
        if longs:
            print(f"{step:22s} | {len(longs)} | {min(longs):.2f} / {float(np.median(longs)):.2f} / {max(longs):.2f}{'':26s} | {flashes}")
        else:
            print(f"{step:22s} | 0 | no native screen of 1 s or more{'':16s} | {flashes}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
