// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { mount } from 'svelte';

import './lib/tokens/tokens.css';
import App from './App.svelte';

const target = document.getElementById('app');
if (!target) {
  throw new Error('missing #app mount point in index.html');
}

const app = mount(App, { target });

export default app;
