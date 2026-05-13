// PoseKeypointsView — render cvitek `pose_keypoints` over a 2D canvas plus
// a 17-row stats table + WS-rate panel.  Drives off the sensingService
// onData stream so it stays in lock-step with the rest of the Sensing tab
// (no duplicate WS connection).

import { sensingService } from '../services/sensing.service.js';

// COCO-17 keypoint order, matches the order used by
// `decode_keypoint_heatmap` in sensing-server.
const COCO = [
  'nose', 'l_eye', 'r_eye', 'l_ear', 'r_ear',
  'l_shoulder', 'r_shoulder', 'l_elbow', 'r_elbow', 'l_wrist', 'r_wrist',
  'l_hip', 'r_hip', 'l_knee', 'r_knee', 'l_ankle', 'r_ankle',
];

// Skeleton edges used to draw the bone lines.
const EDGES = [
  [5, 6], [5, 7], [7, 9], [6, 8], [8, 10],
  [5, 11], [6, 12], [11, 12], [11, 13], [13, 15], [12, 14], [14, 16],
  [0, 5], [0, 6], [0, 1], [0, 2], [1, 3], [2, 4],
];

export class PoseKeypointsView {
  /** @param {HTMLElement} container - the wrapper card element */
  constructor(container) {
    this.container = container;
    this._unsubData = null;
    this._msgCount = 0;
    this._hzStart = performance.now();
    this._hzTimer = null;
  }

  init() {
    this._buildDOM();
    this._cacheElements();
    this._unsubData = sensingService.onData((d) => this._onData(d));

    // Updates/s ticker — recomputed once per second, reset every 5 s so a
    // brief stall does not skew the long-term average.
    this._hzTimer = setInterval(() => {
      const dt = (performance.now() - this._hzStart) / 1000;
      if (dt > 0 && this._hz) this._hz.textContent = (this._msgCount / dt).toFixed(1);
      if (dt > 5) { this._msgCount = 0; this._hzStart = performance.now(); }
    }, 1000);
  }

  dispose() {
    if (this._unsubData) { this._unsubData(); this._unsubData = null; }
    if (this._hzTimer) { clearInterval(this._hzTimer); this._hzTimer = null; }
    this.container.innerHTML = '';
  }

  // ---- DOM construction --------------------------------------------------

  _buildDOM() {
    const rows = COCO.map((n, i) =>
      `<tr><td>${i}</td><td class="pkv-name">${n}</td>` +
      `<td id="pkv-x${i}">—</td><td id="pkv-y${i}">—</td>` +
      `<td id="pkv-c${i}">—</td></tr>`
    ).join('');

    this.container.innerHTML = `
      <div class="sensing-card-title">Pose Keypoints</div>
      <div class="pkv-layout">
        <canvas id="pkv-canvas" class="pkv-canvas" width="320" height="320"></canvas>
        <dl class="pkv-stats">
          <dt>tick</dt>      <dd id="pkv-tick">—</dd>
          <dt>updates/s</dt> <dd id="pkv-hz">—</dd>
          <dt>pose_keypoints</dt><dd id="pkv-len">—</dd>
          <dt>x range</dt>   <dd id="pkv-xrng">—</dd>
          <dt>y range</dt>   <dd id="pkv-yrng">—</dd>
          <dt>conf range</dt><dd id="pkv-crng">—</dd>
        </dl>
      </div>
      <table class="pkv-table">
        <thead>
          <tr><th>#</th><th>name</th><th>x</th><th>y</th><th>conf</th></tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
      <p class="pkv-note">
        Scaffold model with random weights — positions appear synthetic and
        confidence stays near 0.5.  This view validates the inference
        pipeline end-to-end, not pose accuracy.  Meaningful poses require
        a trained model.
      </p>
    `;
  }

  _cacheElements() {
    const $ = (id) => this.container.querySelector('#' + id);
    this._canvas = $('pkv-canvas');
    this._ctx    = this._canvas.getContext('2d');
    this._tick   = $('pkv-tick');
    this._hz     = $('pkv-hz');
    this._len    = $('pkv-len');
    this._xrng   = $('pkv-xrng');
    this._yrng   = $('pkv-yrng');
    this._crng   = $('pkv-crng');
    // Pre-cache row cells — 17×3 = 51 lookups happen per WS frame, worth it.
    this._rows = COCO.map((_, i) => ({
      x: $(`pkv-x${i}`), y: $(`pkv-y${i}`), c: $(`pkv-c${i}`),
    }));
  }

  // ---- Data callback -----------------------------------------------------

  _onData(data) {
    this._msgCount++;
    if (this._tick) this._tick.textContent = data.tick ?? '—';

    const pk = data.pose_keypoints;
    this._len.textContent = Array.isArray(pk) ? pk.length : 'None';

    if (Array.isArray(pk) && pk.length > 0) {
      this._updateRanges(pk);
      this._updateRows(pk);
      this._draw(pk);
    } else {
      this._xrng.textContent = '—';
      this._yrng.textContent = '—';
      this._crng.textContent = '—';
      this._draw(null);
    }
  }

  _updateRanges(pk) {
    const fmt = (arr) =>
      `[${Math.min(...arr).toFixed(2)}, ${Math.max(...arr).toFixed(2)}]`;
    this._xrng.textContent = fmt(pk.map((k) => k[0]));
    this._yrng.textContent = fmt(pk.map((k) => k[1]));
    this._crng.textContent = fmt(pk.map((k) => k[3]));
  }

  _updateRows(pk) {
    for (let i = 0; i < pk.length && i < this._rows.length; i++) {
      const r = this._rows[i], k = pk[i];
      r.x.textContent = k[0].toFixed(3);
      r.y.textContent = k[1].toFixed(3);
      r.c.textContent = k[3].toFixed(3);
    }
  }

  // ---- Canvas render -----------------------------------------------------

  _draw(kps) {
    const ctx = this._ctx;
    const W = this._canvas.width;
    const H = this._canvas.height;
    ctx.clearRect(0, 0, W, H);
    ctx.fillStyle = '#11151b';
    ctx.fillRect(0, 0, W, H);

    // Grid background — 8×8.
    ctx.strokeStyle = '#21262d';
    ctx.lineWidth = 1;
    for (let i = 1; i < 8; i++) {
      const p = i * (W / 8);
      ctx.beginPath(); ctx.moveTo(p, 0); ctx.lineTo(p, H); ctx.stroke();
      ctx.beginPath(); ctx.moveTo(0, p); ctx.lineTo(W, p); ctx.stroke();
    }

    if (!kps) return;

    // Skeleton edges first so joints overpaint the lines.
    ctx.strokeStyle = '#5e81ac';
    ctx.lineWidth = 2;
    for (const [a, b] of EDGES) {
      const A = kps[a], B = kps[b];
      if (!A || !B) continue;
      ctx.beginPath();
      ctx.moveTo(A[0] * W, A[1] * H);
      ctx.lineTo(B[0] * W, B[1] * H);
      ctx.stroke();
    }

    // Joints — colour by index, size scales with conf.
    ctx.font = '10px monospace';
    for (let i = 0; i < kps.length; i++) {
      const [x, y, , c] = kps[i];
      const cx = x * W, cy = y * H;
      const r = 4 + (c || 0) * 4;
      ctx.fillStyle = `hsl(${i * 360 / 17}, 70%, 60%)`;
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill();
      ctx.fillStyle = '#d8dee9';
      ctx.fillText(String(i), cx + 6, cy - 6);
    }
  }
}
