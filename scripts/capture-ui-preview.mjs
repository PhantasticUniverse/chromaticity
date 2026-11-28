// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { Buffer } from 'node:buffer';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const CHROME =
  process.env.CHROMATICITY_CHROME_BIN ||
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const OUT_DIR = path.resolve(process.argv[2] || '.local/ui-captures');
const PREVIEW_PORT = Number(process.env.CHROMATICITY_PREVIEW_PORT || 4173);
const CDP_PORT = Number(process.env.CHROMATICITY_CDP_PORT || 9229);
const PREVIEW_URL = `http://127.0.0.1:${PREVIEW_PORT}/`;

const ROUTES = [
  { rail: 'Perform', file: 'perform.png' },
  { rail: 'Mix', file: 'mix.png' },
  { rail: 'Library', subtab: 'Scenes', file: 'library.png' },
  { rail: 'Author', subtab: 'Editor', file: 'author.png' },
  { rail: 'Author', subtab: 'Audio Uniforms', file: 'audio-uniforms.png' },
  { rail: 'Library', subtab: 'Playlist', file: 'playlist.png' },
  { rail: 'Library', subtab: 'Sources', file: 'sources.png' },
  { rail: 'I/O', subtab: 'MIDI', file: 'midi.png' },
  { rail: 'I/O', subtab: 'OSC', file: 'osc.png' },
  { rail: 'Library', subtab: 'Import', file: 'import.png' },
  { rail: 'Author', subtab: 'Patch', file: 'patch.png' },
  { rail: 'Settings', subtab: 'Preferences', file: 'settings.png' },
  { rail: 'Settings', subtab: 'License', file: 'license.png' },
  { rail: 'Help', file: 'help.png' },
];

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function waitForHttp(url, attempts = 50) {
  for (let i = 0; i < attempts; i += 1) {
    try {
      const res = await fetch(url);
      if (res.ok) return;
    } catch {
      // wait for preview server
    }
    await sleep(200);
  }
  throw new Error(`Timed out waiting for preview server at ${url}`);
}

async function getWsUrl() {
  for (let i = 0; i < 40; i += 1) {
    try {
      const res = await fetch(`http://127.0.0.1:${CDP_PORT}/json`);
      const targets = await res.json();
      const page = targets.find((t) => t.type === 'page' && t.url.startsWith(PREVIEW_URL));
      if (page?.webSocketDebuggerUrl) return page.webSocketDebuggerUrl;
    } catch {
      // wait for Chrome DevTools target
    }
    await sleep(200);
  }
  throw new Error(`Timed out waiting for Chrome DevTools target on port ${CDP_PORT}`);
}

async function main() {
  fs.mkdirSync(OUT_DIR, { recursive: true });

  const preview = spawn(
    'npx',
    ['vite', 'preview', '--host', '127.0.0.1', '--port', String(PREVIEW_PORT), '--strictPort'],
    { stdio: 'ignore' },
  );

  const userDataDir = path.join(OUT_DIR, `.chrome-profile-${Date.now()}`);
  let chrome = null;

  try {
    await waitForHttp(PREVIEW_URL);

    chrome = spawn(
      CHROME,
      [
        '--headless=new',
        `--remote-debugging-port=${CDP_PORT}`,
        `--user-data-dir=${userDataDir}`,
        '--window-size=1440,900',
        '--force-device-scale-factor=2',
        '--disable-gpu-sandbox',
        '--no-first-run',
        '--no-default-browser-check',
        PREVIEW_URL,
      ],
      { stdio: 'ignore' },
    );

    const wsUrl = await getWsUrl();
    const ws = new WebSocket(wsUrl);
    await new Promise((resolve, reject) => {
      ws.addEventListener('open', resolve, { once: true });
      ws.addEventListener('error', reject, { once: true });
    });

    let msgId = 1;
    const pending = new Map();
    ws.addEventListener('message', (ev) => {
      const data = JSON.parse(ev.data.toString());
      if (data.id && pending.has(data.id)) {
        const { resolve, reject } = pending.get(data.id);
        pending.delete(data.id);
        if (data.error) reject(new Error(data.error.message));
        else resolve(data.result);
      }
    });

    const send = (method, params = {}) =>
      new Promise((resolve, reject) => {
        const id = msgId++;
        pending.set(id, { resolve, reject });
        ws.send(JSON.stringify({ id, method, params }));
      });

    await send('Page.enable');
    await send('Emulation.setDeviceMetricsOverride', {
      width: 1440,
      height: 900,
      deviceScaleFactor: 2,
      mobile: false,
    });
    await sleep(900);

    for (const route of ROUTES) {
      await send('Runtime.evaluate', {
        expression: `(() => {
          const btn = document.querySelector('button.sidebar-rail__btn[title="${route.rail}"]');
          if (btn) btn.click();
          return !!btn;
        })()`,
        returnByValue: true,
      });
      if (route.subtab) {
        await sleep(180);
        await send('Runtime.evaluate', {
          expression: `(() => {
            const tabs = Array.from(document.querySelectorAll('button.app-shell__subtab'));
            const t = tabs.find((b) => b.textContent?.trim() === ${JSON.stringify(route.subtab)});
            if (t) t.click();
            const main = document.querySelector('main, .app-shell__view');
            if (main) main.scrollTop = 0;
            return !!t;
          })()`,
          returnByValue: true,
        });
      }
      await sleep(550);
      const { data } = await send('Page.captureScreenshot', { format: 'png' });
      const outPath = path.join(OUT_DIR, route.file);
      fs.writeFileSync(outPath, Buffer.from(data, 'base64'));
      console.log(`Captured ${route.file}`);
    }

    // Perform Console (3-column) variant
    await send('Runtime.evaluate', {
      expression: `(() => {
        const btn = document.querySelector('button.sidebar-rail__btn[title="Perform"]');
        if (btn) btn.click();
      })()`,
    });
    await sleep(350);
    const hasConsole = await send('Runtime.evaluate', {
      expression: `(() => {
        const btns = Array.from(document.querySelectorAll('button'));
        const c = btns.find((b) => b.textContent?.trim().toUpperCase() === 'CONSOLE');
        if (c) { c.click(); return true; }
        return false;
      })()`,
      returnByValue: true,
    });
    if (hasConsole?.result?.value) {
      await sleep(450);
      const { data } = await send('Page.captureScreenshot', { format: 'png' });
      fs.writeFileSync(path.join(OUT_DIR, 'perform-console.png'), Buffer.from(data, 'base64'));
      console.log('Captured perform-console.png');
    }

    // Stage Dock open on Author view
    await send('Runtime.evaluate', {
      expression: `(() => {
        const btn = document.querySelector('button.sidebar-rail__btn[title="Author"]');
        if (btn) btn.click();
      })()`,
    });
    await sleep(350);
    const hasStage = await send('Runtime.evaluate', {
      expression: `(() => {
        const btns = Array.from(document.querySelectorAll('header button, .top-bar button'));
        const btn = btns.find((b) => b.textContent?.trim().toUpperCase().includes('STAGE'));
        if (btn) { btn.click(); return true; }
        return false;
      })()`,
      returnByValue: true,
    });
    if (hasStage?.result?.value) {
      await sleep(450);
      const { data } = await send('Page.captureScreenshot', { format: 'png' });
      fs.writeFileSync(path.join(OUT_DIR, 'stage-dock.png'), Buffer.from(data, 'base64'));
      console.log('Captured stage-dock.png');
    }

    ws.close();
  } finally {
    if (chrome) chrome.kill('SIGTERM');
    preview.kill('SIGTERM');
    try {
      fs.rmSync(userDataDir, { recursive: true, force: true });
    } catch {
      // Chrome may still be flushing profile files on exit
    }
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
