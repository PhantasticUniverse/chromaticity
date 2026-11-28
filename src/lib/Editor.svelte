<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  /*
   * CodeMirror 6 editor wrapper for the W21 PR3 Author view. Mounts a
   * single `EditorView` per language; the parent component swaps the
   * `language` / `value` props when the user picks a different file
   * tab, and the wrapper rebuilds the view in place.
   *
   * Design choices:
   *   - One view per `<Editor>` mount. Tab switching at the parent
   *     destroys + recreates the wrapper, which is correct for the
   *     read-only tabs (config.json / readme.md) and acceptable for
   *     the writeable ones (main.glsl / script.js) since the parent
   *     also retains buffer state in `$state`.
   *   - Theme is built as a CodeMirror `EditorView.theme` object that
   *     references `tokens.css` CSS custom properties via `var(--…)`.
   *     This means the theme cycle (Pulse / Atelier / Console) works
   *     by virtue of the CSS cascade — no per-theme JS rebuild.
   *   - Autocomplete is enabled only for the GLSL language (the brief
   *     does not call for completion in script.js / config.json /
   *     readme.md). A custom `CompletionSource` is wired in the GLSL
   *     branch only.
   *   - SSR is not a concern: this app is a Tauri 2 SPA and Vite
   *     dev/build only emits client-side bundles. CodeMirror's DOM
   *     reads at construction time are safe inside the Svelte 5
   *     `$effect` lifecycle.
   *
   * Per the W21 PR3 brief, this is a controlled divergence from the
   * v0.2 bundle's `<pre><code>` editor — the bundle never integrated
   * CodeMirror, but operator-quality authoring requires real editing.
   */

  import {
    autocompletion,
    type CompletionContext,
    type CompletionResult,
  } from '@codemirror/autocomplete';
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
  import { cpp } from '@codemirror/lang-cpp';
  import { javascript } from '@codemirror/lang-javascript';
  import { json } from '@codemirror/lang-json';
  import { markdown } from '@codemirror/lang-markdown';
  import {
    bracketMatching,
    foldGutter,
    foldKeymap,
    indentOnInput,
    syntaxHighlighting,
    HighlightStyle,
  } from '@codemirror/language';
  import { Compartment, EditorState, type Extension } from '@codemirror/state';
  import {
    drawSelection,
    EditorView,
    highlightActiveLine,
    highlightActiveLineGutter,
    keymap,
    lineNumbers,
  } from '@codemirror/view';
  import { tags as t } from '@lezer/highlight';
  import { untrack } from 'svelte';

  import { GLSL_COMPLETIONS, categoryBoost, categoryType } from './data/glsl-completions';

  /** Supported language modes — maps 1:1 to the Author file tabs. */
  export type EditorLanguage = 'glsl' | 'js' | 'json' | 'md';

  type Props = {
    /** Initial document contents. The wrapper takes ownership of the
     *  buffer; the parent observes changes via `onChange`. */
    value: string;
    /** Active language. Switching language at runtime is supported via
     *  a CodeMirror `Compartment` swap. */
    language: EditorLanguage;
    /** ARIA label routed through to the EditorView's content DOM. */
    ariaLabel: string;
    /** Read-only flag. The autosave UX uses this for the diff pane. */
    readOnly?: boolean;
    /** Notified on every doc change with the new full text. */
    onChange?: (next: string) => void;
  };

  const { value, language, ariaLabel, readOnly = false, onChange }: Props = $props();

  let host: HTMLDivElement | null = $state(null);
  let view: EditorView | null = null;
  const languageCompartment = new Compartment();
  const readOnlyCompartment = new Compartment();
  const ariaLabelCompartment = new Compartment();

  function languageExtension(lang: EditorLanguage): Extension {
    if (lang === 'glsl') {
      // GLSL syntax is C-like; lang-cpp's tokenizer is the closest
      // first-party CodeMirror language pack and renders GLSL
      // keywords / types correctly. The autocomplete vocabulary is
      // attached separately as a CompletionSource.
      return [cpp(), autocompletion({ override: [glslCompletion] })];
    }
    if (lang === 'js') {
      return javascript();
    }
    if (lang === 'json') {
      return json();
    }
    return markdown();
  }

  /** CodeMirror highlight style hooked to tokens.css custom properties.
   *  The legacy regex tokeniser used `--color-signal-magenta` for
   *  keywords, `--color-signal-cyan` for uniforms, `--color-signal-orange`
   *  for numbers, and `--color-signal-lime` for strings; preserve that
   *  mapping so the visual feel before/after the CodeMirror swap stays
   *  consistent across the theme cycle. */
  const highlightStyle = HighlightStyle.define([
    { tag: t.comment, color: 'var(--color-text-3)', fontStyle: 'italic' },
    { tag: t.lineComment, color: 'var(--color-text-3)', fontStyle: 'italic' },
    { tag: t.blockComment, color: 'var(--color-text-3)', fontStyle: 'italic' },
    { tag: t.docComment, color: 'var(--color-text-3)', fontStyle: 'italic' },
    { tag: t.keyword, color: 'var(--color-signal-magenta)' },
    { tag: t.controlKeyword, color: 'var(--color-signal-magenta)' },
    { tag: t.modifier, color: 'var(--color-signal-magenta)' },
    { tag: t.definitionKeyword, color: 'var(--color-signal-magenta)' },
    { tag: t.operatorKeyword, color: 'var(--color-signal-magenta)' },
    { tag: t.typeName, color: 'var(--color-signal-cyan)' },
    { tag: t.standard(t.typeName), color: 'var(--color-signal-cyan)' },
    { tag: t.variableName, color: 'var(--color-text-1)' },
    { tag: t.propertyName, color: 'var(--color-text-1)' },
    { tag: t.function(t.variableName), color: 'var(--color-signal-cyan)' },
    { tag: t.macroName, color: 'var(--color-signal-violet)' },
    { tag: t.processingInstruction, color: 'var(--color-signal-violet)' },
    { tag: t.number, color: 'var(--color-signal-orange)' },
    { tag: t.string, color: 'var(--color-signal-lime)' },
    { tag: t.special(t.string), color: 'var(--color-signal-lime)' },
    { tag: t.bool, color: 'var(--color-signal-orange)' },
    { tag: t.null, color: 'var(--color-signal-orange)' },
    { tag: t.atom, color: 'var(--color-signal-orange)' },
    { tag: t.heading, color: 'var(--color-text-0)', fontWeight: '600' },
    { tag: t.link, color: 'var(--color-signal-cyan)', textDecoration: 'underline' },
    { tag: t.emphasis, fontStyle: 'italic' },
    { tag: t.strong, fontWeight: '600' },
  ]);

  /** Token-driven theme. All colours/spacings reference tokens.css so
   *  Pulse / Atelier / Console cycle without rebuilding the view. */
  const chromaticityTheme = EditorView.theme(
    {
      '&': {
        color: 'var(--color-text-1)',
        backgroundColor: 'var(--color-bg-1)',
        height: '100%',
        fontSize: 'var(--font-size-mono-sm)',
        fontFamily: 'var(--font-family-mono)',
      },
      '.cm-content': {
        caretColor: 'var(--color-primary)',
        padding: 'var(--space-sm) 0',
      },
      '.cm-cursor, .cm-dropCursor': {
        borderLeftColor: 'var(--color-primary)',
      },
      '&.cm-focused .cm-selectionBackground, ::selection': {
        backgroundColor: 'color-mix(in oklab, var(--color-primary) 22%, transparent)',
      },
      '.cm-gutters': {
        backgroundColor: 'var(--color-bg-1)',
        color: 'var(--color-text-4)',
        border: 'none',
        borderRight: '1px solid var(--color-line-1)',
        fontFamily: 'var(--font-family-mono)',
      },
      '.cm-activeLineGutter': {
        backgroundColor: 'var(--color-bg-2)',
        color: 'var(--color-text-2)',
      },
      '.cm-activeLine': {
        backgroundColor: 'color-mix(in oklab, var(--color-bg-2) 60%, transparent)',
      },
      '.cm-foldGutter .cm-gutterElement': {
        color: 'var(--color-text-4)',
      },
      '.cm-tooltip': {
        backgroundColor: 'var(--color-bg-3)',
        color: 'var(--color-text-1)',
        border: '1px solid var(--color-line-2)',
        borderRadius: 'var(--radius-sm)',
        fontFamily: 'var(--font-family-sans)',
        fontSize: 'var(--font-size-body-sm)',
        boxShadow: 'var(--shadow-panel)',
      },
      '.cm-tooltip.cm-tooltip-autocomplete > ul': {
        fontFamily: 'var(--font-family-mono)',
        fontSize: 'var(--font-size-mono-sm)',
        maxHeight: '260px',
      },
      '.cm-tooltip-autocomplete ul li': {
        padding: '2px var(--space-sm)',
      },
      '.cm-tooltip-autocomplete ul li[aria-selected]': {
        backgroundColor: 'color-mix(in oklab, var(--color-primary) 18%, transparent)',
        color: 'var(--color-text-0)',
      },
      '.cm-completionLabel': {
        color: 'var(--color-text-1)',
      },
      '.cm-completionDetail': {
        color: 'var(--color-text-3)',
        fontStyle: 'normal',
        marginLeft: 'var(--space-sm)',
      },
      '.cm-completionInfo': {
        backgroundColor: 'var(--color-bg-3)',
        color: 'var(--color-text-1)',
        border: '1px solid var(--color-line-2)',
        borderRadius: 'var(--radius-sm)',
        padding: 'var(--space-sm) var(--space-md)',
        maxWidth: '320px',
      },
      // Custom badges per category, applied via the `.cm-completionIcon` slot.
      '.cm-completionIcon-chromaNative::after': {
        content: '"chroma"',
      },
      '.cm-completionMatchedText': {
        color: 'var(--color-primary)',
        textDecoration: 'none',
      },
      '.cm-scroller': {
        fontFamily: 'var(--font-family-mono)',
        lineHeight: '1.55',
      },
    },
    { dark: true },
  );

  /** Autocomplete `CompletionSource` for the GLSL editor. CodeMirror
   *  hands us a context describing the cursor + active token; we
   *  return a list of `Completion` objects that match the prefix.
   *  The dropdown sort is driven by the `boost` field (higher first);
   *  the `type` field selects the CodeMirror builtin icon and the
   *  `detail` text appears next to the label. */
  function glslCompletion(context: CompletionContext): CompletionResult | null {
    const word = context.matchBefore(/[\w_]+/);
    if (word === null) return null;
    if (word.from === word.to && !context.explicit) return null;
    return {
      from: word.from,
      to: word.to,
      options: GLSL_COMPLETIONS.map((c) => {
        const opt: {
          label: string;
          type: string;
          detail: string;
          boost: number;
          info?: string;
        } = {
          label: c.label,
          type: categoryType(c.category),
          detail: c.detail,
          boost: categoryBoost(c.category),
        };
        if (c.info !== undefined) opt.info = c.info;
        return opt;
      }),
      validFor: /^[\w_]*$/,
    };
  }

  function buildExtensions(lang: EditorLanguage, ro: boolean, label: string): Extension[] {
    return [
      lineNumbers(),
      foldGutter(),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      drawSelection(),
      indentOnInput(),
      bracketMatching(),
      history(),
      syntaxHighlighting(highlightStyle),
      keymap.of([...defaultKeymap, ...historyKeymap, ...foldKeymap, indentWithTab]),
      languageCompartment.of(languageExtension(lang)),
      readOnlyCompartment.of([EditorState.readOnly.of(ro), EditorView.editable.of(!ro)]),
      chromaticityTheme,
      ariaLabelCompartment.of(EditorView.contentAttributes.of({ 'aria-label': label })),
      EditorView.updateListener.of((u) => {
        if (u.docChanged && onChange) {
          onChange(u.state.doc.toString());
        }
      }),
    ];
  }

  $effect(() => {
    if (host === null) return;
    const initialValue = untrack(() => value);
    const initialLanguage = untrack(() => language);
    const initialReadOnly = untrack(() => readOnly);
    const initialAriaLabel = untrack(() => ariaLabel);
    view = new EditorView({
      doc: initialValue,
      parent: host,
      extensions: buildExtensions(initialLanguage, initialReadOnly, initialAriaLabel),
    });
    return () => {
      view?.destroy();
      view = null;
    };
  });

  // Reactively sync external value changes without rebuilding the
  // EditorView. Normal typing already updates the parent to the same
  // string, so this is a no-op during ordinary edits and preserves the
  // cursor/selection/history stack.
  $effect(() => {
    if (view === null) return;
    setDoc(value);
  });

  // Reactively reconfigure language compartment when the prop changes,
  // without destroying the editor (preserves cursor / selection state).
  $effect(() => {
    if (view === null) return;
    view.dispatch({
      effects: languageCompartment.reconfigure(languageExtension(language)),
    });
  });

  // Reactively reconfigure read-only compartment.
  $effect(() => {
    if (view === null) return;
    view.dispatch({
      effects: readOnlyCompartment.reconfigure([
        EditorState.readOnly.of(readOnly),
        EditorView.editable.of(!readOnly),
      ]),
    });
  });

  // Reactively reconfigure the editable content's ARIA label without
  // remounting the editor.
  $effect(() => {
    if (view === null) return;
    view.dispatch({
      effects: ariaLabelCompartment.reconfigure(
        EditorView.contentAttributes.of({ 'aria-label': ariaLabel }),
      ),
    });
  });

  /** External: replace the buffer text. The Author view calls this on
   *  Revert / Load Backup so the editor's view of the document stays
   *  authoritative through reactive prop changes. */
  export function setDoc(next: string): void {
    if (view === null) return;
    const current = view.state.doc.toString();
    if (current === next) return;
    view.dispatch({
      changes: { from: 0, to: current.length, insert: next },
    });
  }
</script>

<div bind:this={host} class="chr-editor" data-language={language}></div>

<style>
  .chr-editor {
    display: block;
    width: 100%;
    height: 100%;
    min-height: 280px;
    background: var(--color-bg-1);
    border: 0;
    overflow: hidden;
  }
  /* CodeMirror's content area; ensure scrolling is contained inside
     the host so the surrounding panel chrome stays static. */
  .chr-editor :global(.cm-editor) {
    height: 100%;
    outline: none;
  }
  .chr-editor :global(.cm-editor.cm-focused) {
    outline: none;
  }
  .chr-editor :global(.cm-scroller) {
    overflow: auto;
  }
</style>
