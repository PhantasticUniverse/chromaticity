// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { mount } from 'svelte';

import './lib/tokens/tokens.css';
import App from './App.svelte';
import { inspectAllStateMachines } from './lib/state';

const target = document.getElementById('app');
if (!target) {
  throw new Error('missing #app mount point in index.html');
}

// Read-only state-machine diagnostics hook for DevTools / agent audits (DX + AX).
if (typeof window !== 'undefined') {
  Object.defineProperty(window, '__CHROMATICITY_STATE__', {
    value: inspectAllStateMachines,
    configurable: true,
    enumerable: false,
    writable: false,
  });
}

const app = mount(App, { target });

export default app;
