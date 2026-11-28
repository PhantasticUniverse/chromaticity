// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { describe, expect, it } from 'vitest';

import {
  closeOutputWindow,
  getOutputWindowStatus,
  listOutputMonitors,
  openOutputWindow,
  toggleOutputFullscreen,
} from './output';

describe('output IPC browser fallbacks', () => {
  it('enumerates a fallback display and round-trips open/fullscreen/close', async () => {
    const monitors = await listOutputMonitors();
    expect(monitors.length).toBeGreaterThanOrEqual(1);
    expect(monitors[0]?.is_primary).toBe(true);

    const opened = await openOutputWindow({ fullscreen: false });
    expect(opened.open).toBe(true);
    expect(opened.fullscreen).toBe(false);

    const fs = await toggleOutputFullscreen();
    expect(fs.fullscreen).toBe(true);

    const closed = await closeOutputWindow();
    expect(closed.open).toBe(false);

    const status = await getOutputWindowStatus();
    expect(status.open).toBe(false);
  });
});

describe('output display setting (audit 2026-11-28)', () => {
  it('maps the persisted display setting to a monitor index for Go Live', async () => {
    const { monitorIndexForDisplaySetting, displaySettingForMonitor, DISPLAY_AUTO } =
      await import('./output');
    expect(monitorIndexForDisplaySetting(DISPLAY_AUTO)).toBeUndefined();
    expect(monitorIndexForDisplaySetting('display-1')).toBe(0);
    expect(monitorIndexForDisplaySetting('display-3')).toBe(2);
    expect(monitorIndexForDisplaySetting('display-0')).toBeUndefined();
    expect(monitorIndexForDisplaySetting('garbage')).toBeUndefined();
    expect(displaySettingForMonitor(1)).toBe('display-2');
  });
});
