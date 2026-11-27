// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W63 Editor wrapper tests — direct coverage of `src/lib/Editor.svelte`,
// the W21 PR3 CodeMirror 6 wrapper consumed by the Author route view.
// The shell-component test track was claimed closed at W51/W52 (every
// file in `src/lib/*.svelte` paired with a `.test.ts`); this suite
// closes the residual gap (Editor was the only un-paired file under
// `src/lib/*.svelte`) by covering the wrapper's externally-observable
// surface directly rather than transitively through Author.
//
// Surface under test (the props declared on Editor.svelte):
//
//   * value:      initial document; round-trips into `.cm-content`.
//   * language:   `'glsl' | 'js' | 'json' | 'md'` — drives the
//                 `languageCompartment` extension and is mirrored on the
//                 host's `data-language` attribute.
//   * ariaLabel:  routed to the EditorView content DOM via
//                 `EditorView.contentAttributes`.
//   * readOnly:   forwarded into `EditorState.readOnly` +
//                 `EditorView.editable`; the `.cm-content` exposes
//                 `contenteditable="false"` when readOnly is true.
//   * onChange:   fired with the new full text on every doc-changed
//                 update; not fired on remount.
//
// Non-obvious findings surfaced while authoring this suite:
//
//  * **CodeMirror 6 mounts cleanly under jsdom** (this is the same
//    finding W60 surfaced through the Author transitive path; it
//    holds when Editor is mounted directly too — no
//    `vi.mock('@codemirror/...')` shim required, no DOM-rect stubs,
//    no rAF plumbing). The constructor reads layout attributes but
//    none of those calls throw.
//  * **`onChange` is fired by dispatching a CodeMirror transaction**.
//    The wrapper installs an `EditorView.updateListener` that fires
//    `onChange(state.doc.toString())` whenever `update.docChanged`
//    is true; we drive that path by reading the cm-content's
//    `cm-line` text via `dispatchEvent` of an `input` event isn't
//    reliable under jsdom (CodeMirror's input bridge depends on
//    `beforeinput` / IME plumbing not implemented in jsdom). Instead
//    we exercise the external prop-sync path with Testing Library's
//    `rerender()`, asserting that the same EditorView receives the
//    replacement text through dispatch.
//  * **`value` prop changes sync through CodeMirror dispatch** rather
//    than through an EditorView remount. Author still re-keys across
//    file-tab changes, but ordinary buffer edits should preserve the
//    CodeMirror instance, cursor, selection, and undo stack.
//  * **CodeMirror normalises whitespace + line breaks into per-line
//    `<div class="cm-line">` rows.** A multi-line `value` round-trip
//    is asserted by joining `.cm-line` `.textContent` rather than
//    by querying for the raw newline.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.
// No IPC, no `matchMedia`, no fixture imports — Editor is a leaf
// component with a pure prop surface. The standard `cleanup()` is
// the only teardown required.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import Editor from './Editor.svelte';

/** Read the CodeMirror content text by joining per-line `.cm-line`
 *  rows. Direct `.cm-content` `textContent` collapses newlines into
 *  contiguous strings; per-line joining preserves line boundaries. */
function readDoc(container: HTMLElement): string {
  const lines = container.querySelectorAll('.cm-content .cm-line');
  return Array.from(lines)
    .map((el) => el.textContent ?? '')
    .join('\n');
}

describe('Editor (CodeMirror 6 wrapper)', () => {
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('mounts a CodeMirror EditorView inside the host element', () => {
    const { container } = render(Editor, {
      props: {
        value: '// hello',
        language: 'js',
        ariaLabel: 'main.glsl source',
      },
    });
    const host = container.querySelector('.chr-editor');
    expect(host).not.toBeNull();
    // CodeMirror appends `.cm-editor` to its parent host as part of
    // the EditorView constructor; its presence confirms the view
    // mounted under jsdom.
    expect(container.querySelector('.cm-editor')).not.toBeNull();
    expect(container.querySelector('.cm-content')).not.toBeNull();
  });

  it('mirrors the language prop on the host data-language attribute', () => {
    const { container } = render(Editor, {
      props: {
        value: '',
        language: 'glsl',
        ariaLabel: 'main.glsl source',
      },
    });
    const host = container.querySelector('.chr-editor');
    expect(host).toHaveAttribute('data-language', 'glsl');
  });

  it('round-trips the initial `value` prop into the editor content', () => {
    const initial = 'void main() {\n  gl_FragColor = vec4(1.0);\n}';
    const { container } = render(Editor, {
      props: {
        value: initial,
        language: 'glsl',
        ariaLabel: 'main.glsl source',
      },
    });
    expect(readDoc(container)).toBe(initial);
  });

  it('routes the ariaLabel prop onto the cm-content node', () => {
    const { container } = render(Editor, {
      props: {
        value: '',
        language: 'json',
        ariaLabel: 'config.json source',
      },
    });
    const content = container.querySelector('.cm-content');
    expect(content).not.toBeNull();
    expect(content).toHaveAttribute('aria-label', 'config.json source');
  });

  it('sets contenteditable=false on the content node when readOnly is true', () => {
    const { container } = render(Editor, {
      props: {
        value: '# Read me',
        language: 'md',
        ariaLabel: 'readme.md source',
        readOnly: true,
      },
    });
    const content = container.querySelector('.cm-content');
    expect(content).not.toBeNull();
    expect(content).toHaveAttribute('contenteditable', 'false');
  });

  it('defaults readOnly to false (editable cm-content)', () => {
    const { container } = render(Editor, {
      props: {
        value: '# Read me',
        language: 'md',
        ariaLabel: 'readme.md source',
      },
    });
    const content = container.querySelector('.cm-content');
    expect(content).not.toBeNull();
    expect(content).toHaveAttribute('contenteditable', 'true');
  });

  it('renders gutters and line numbers as part of the bundled extensions', () => {
    const { container } = render(Editor, {
      props: {
        value: 'line one\nline two\nline three',
        language: 'js',
        ariaLabel: 'script.js source',
      },
    });
    // `lineNumbers()` extension is wired in `buildExtensions`; the
    // gutter element is created on first render. Three lines = three
    // line-number elements rendered in the line-numbers gutter.
    const gutters = container.querySelector('.cm-gutters');
    expect(gutters).not.toBeNull();
    const lineNumberGutter = container.querySelector('.cm-lineNumbers');
    expect(lineNumberGutter).not.toBeNull();
    const lineNumberCells = container.querySelectorAll('.cm-lineNumbers .cm-gutterElement');
    // The first cell is the gutter spacer; per CodeMirror conventions
    // the user-visible numbered cells are the remaining ones.
    expect(lineNumberCells.length).toBeGreaterThanOrEqual(3);
  });

  it('mounts each of the four supported languages without throwing', () => {
    for (const language of ['glsl', 'js', 'json', 'md'] as const) {
      const { container, unmount } = render(Editor, {
        props: {
          value: '',
          language,
          ariaLabel: `${language} source`,
        },
      });
      const host = container.querySelector('.chr-editor');
      expect(host).not.toBeNull();
      expect(host).toHaveAttribute('data-language', language);
      expect(container.querySelector('.cm-editor')).not.toBeNull();
      unmount();
    }
  });

  it('does not call `onChange` on the initial mount', () => {
    const onChange = vi.fn();
    render(Editor, {
      props: {
        value: 'initial buffer',
        language: 'js',
        ariaLabel: 'script.js source',
        onChange,
      },
    });
    // The `EditorView.updateListener.of((u) => { if (u.docChanged) onChange(...) })`
    // contract guarantees no fire on construct (the constructor's
    // first transaction sets `docChanged=false`).
    expect(onChange).not.toHaveBeenCalled();
  });

  it('syncs a value prop change without remounting CodeMirror', async () => {
    const { container, rerender } = render(Editor, {
      props: {
        value: 'first buffer',
        language: 'js',
        ariaLabel: 'script.js source',
      },
    });
    const mountedView = container.querySelector('.cm-editor');
    expect(mountedView).not.toBeNull();

    await rerender({
      value: 'second buffer',
      language: 'js',
      ariaLabel: 'script.js source',
    });

    expect(container.querySelector('.cm-editor')).toBe(mountedView);
    expect(readDoc(container)).toBe('second buffer');
  });

  it('cleans up the EditorView on unmount (no leaked .cm-editor host)', () => {
    const { container, unmount } = render(Editor, {
      props: {
        value: '// hi',
        language: 'js',
        ariaLabel: 'script.js source',
      },
    });
    expect(container.querySelector('.cm-editor')).not.toBeNull();
    unmount();
    // The `$effect` teardown calls `view.destroy()`; CodeMirror
    // removes its appended `.cm-editor` from the DOM as part of
    // destroy(). The host `.chr-editor` is removed too because the
    // Svelte component itself unmounts.
    expect(container.querySelector('.cm-editor')).toBeNull();
    expect(container.querySelector('.chr-editor')).toBeNull();
  });

  it('renders an empty document when `value` is the empty string', () => {
    const { container } = render(Editor, {
      props: {
        value: '',
        language: 'js',
        ariaLabel: 'script.js source',
      },
    });
    // CodeMirror still renders a single empty `.cm-line` row for an
    // empty doc, so `readDoc()` returns the empty string.
    expect(readDoc(container)).toBe('');
    // And the `.cm-content` exists, just with no character content.
    const content = container.querySelector('.cm-content');
    expect(content).not.toBeNull();
  });

  it('places the host inside the chr-editor scoped wrapper (CSS contract)', () => {
    const { container } = render(Editor, {
      props: {
        value: '',
        language: 'glsl',
        ariaLabel: 'main.glsl source',
      },
    });
    // The Editor.svelte `<style>` block scopes `.cm-editor` /
    // `.cm-scroller` rules under `.chr-editor :global(...)`. Sanity-
    // check that the cm-editor is a descendant of the chr-editor host
    // so those scoped rules apply.
    const host = container.querySelector('.chr-editor');
    const cm = container.querySelector('.cm-editor');
    expect(host).not.toBeNull();
    expect(cm).not.toBeNull();
    if (host === null || cm === null) return;
    expect(host.contains(cm)).toBe(true);
  });
});
