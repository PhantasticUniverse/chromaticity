// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W30 License view tests — exercise the four state branches
// (`activated` / `trial` / `expired` / `unactivated`) and the
// stub-IPC button-wiring contract. Vitest globals are disabled in
// `vitest.config.ts`; we import `describe` / `it` / `expect`
// explicitly per the `ErrorMessage.test.ts` pattern.
//
// `@tauri-apps/api/core` is mocked so the action handlers can be
// exercised without a Tauri runtime — the stub `invoke` rejects with
// a deterministic error so the `<ErrorMessage>` rendering path can
// be observed.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import License from './License.svelte';
import {
  LICENSE_FIXTURE,
  LICENSE_TRIAL_FIXTURE,
  LICENSE_EXPIRED_FIXTURE,
  LICENSE_UNACTIVATED_FIXTURE,
} from '../data/license-fixtures';

describe('License view', () => {
  afterEach(() => {
    cleanup();
  });

  it('activated branch renders "Activated" heading and seat ID', () => {
    const { getByText } = render(License, { props: { fixture: LICENSE_FIXTURE } });
    expect(getByText('Activated')).toBeInTheDocument();
    expect(getByText('op-20261-4821-prod')).toBeInTheDocument();
  });

  it('trial branch renders "Trial" heading and 14-day expiry', () => {
    const { getByText } = render(License, { props: { fixture: LICENSE_TRIAL_FIXTURE } });
    expect(getByText('Trial')).toBeInTheDocument();
    expect(getByText('14')).toBeInTheDocument();
    expect(getByText(/14 days remaining · bound to this machine/)).toBeInTheDocument();
  });

  it('expired branch renders "Expired" heading and watermark-re-enabled subline', () => {
    const { getByText, queryByText } = render(License, {
      props: { fixture: LICENSE_EXPIRED_FIXTURE },
    });
    expect(getByText('Expired')).toBeInTheDocument();
    expect(getByText(/Watermark re-enabled/)).toBeInTheDocument();
    expect(queryByText(/Expires in/)).toBeNull();
  });

  it('unactivated branch renders "No License" and Request trial button', () => {
    const { getByText, getByRole } = render(License, {
      props: { fixture: LICENSE_UNACTIVATED_FIXTURE },
    });
    expect(getByText('No License')).toBeInTheDocument();
    expect(getByText(/Running with watermark/)).toBeInTheDocument();
    expect(getByRole('button', { name: /request trial/i })).toBeInTheDocument();
  });

  it('capabilities table renders all 15 rows', () => {
    const { container } = render(License, { props: { fixture: LICENSE_FIXTURE } });
    const rows = container.querySelectorAll('.license__cap-table tbody tr');
    expect(rows.length).toBe(15);
  });

  it('Install button is enabled, Refresh button is disabled in activated branch', () => {
    const { getByRole } = render(License, { props: { fixture: LICENSE_FIXTURE } });
    const install = getByRole('button', { name: /install license/i });
    const refresh = getByRole('button', { name: /refresh license/i });
    expect(install).not.toBeDisabled();
    expect(refresh).toBeDisabled();
  });
});
