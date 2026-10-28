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

Alignment: the first video frame of each episode lags the group's first dial report by a roughly
constant amount (camera + display latency), so that constant is estimated as the median over all
episodes. The ending frame is then the first log FRAME written at or after the episode's end.
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
    # Dial groups: a report >= 5 s after the previous one starts a group (the native screen lasts
    # about 4.5-6 s after the last report, so a later report is a new episode in practice; the
    # lab itself groups at 10 s, which merges nothing the video can tell apart because the user
    # waits for the clock between turns).
    gstarts, glasts = [], []
    for t in dials:
        if not glasts or t - glasts[-1] >= 5.0:
            gstarts.append(t)
            glasts.append(t)
        else:
            glasts[-1] = t
    if not eps or not gstarts:
        print("no episodes or no dial groups to align")
        return 1
    # The recording usually starts while the stock screen is still up (before GxOled draws):
    # drop leading episodes that have no dial group.
    extra = max(0, len(eps) - len(gstarts))
    if extra:
        print(f"note: ignoring {extra} leading episode(s) that precede the first dial (stock screen at start)")
        eps = eps[extra:]
    pairs = min(len(eps), len(gstarts))
    offset = float(np.median([eps[i][0] - gstarts[i] for i in range(pairs)]))
    print(f"{len(gstarts)} dial group(s) in the log; video-start minus first-dial (median) = {offset:.3f} s")
    if len(eps) != len(gstarts):
        print(f"WARNING: {len(eps)} episodes in the video vs {len(gstarts)} dial groups in the log; aligning in order")
    print(f"per-episode start offsets: {[round(float(eps[i][0] - gstarts[i]), 3) for i in range(pairs)]}")
    labels = {round(t, 3): s for t, s in groups}
    print()
    print("group | step                 | video OSD s | visible after last dial s | ending frame (seq @ s after last dial)")
    for i in range(pairs):
        a, b = eps[i]
        end_log = b - offset
        later = [(t, seq, step) for t, seq, step in log_frames if t >= end_log - 0.15]
        ending = later[0] if later else None
        label = "-"
        for t, s in groups:
            if abs(t - gstarts[i]) < 0.05:
                label = s
        visible_last = end_log - glasts[i]
        end_txt = f"seq {ending[1]} @ {ending[0] - glasts[i]:.2f}" if ending else "n/a"
        print(f"{i:5d} | {label:20s} | {b - a:11.2f} | {visible_last:25.2f} | {end_txt}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
