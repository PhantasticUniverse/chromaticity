// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Owned, host-side projection of a script execution result.
//!
//! [`ScriptValue`] is the lowering target for the QuickJS `Value`
//! returned by [`crate::ScriptRuntime::execute`]. The script-side
//! `Value` is bound by an `'js` lifetime parameter tied to the
//! `Ctx` it was created in; lifting the result to a borrow-free
//! owned tree lets callers hold the result outside the
//! `Context::with` closure that drives QuickJS evaluation.
//!
//! Slice-1 vocabulary: `Null` / `Bool` / `Number` / `String` /
//! `Array` / `Object`. `undefined` is mapped to `Null` (the loss
//! of fidelity is intentional — slice-1 callers don't differentiate
//! between the two and JSON-style projection is the easier idiom
//! for the contract corpus). `Symbol`, `BigInt`, `Function`, and
//! `Promise` are intentionally NOT representable; the caller
//! receives [`crate::ScriptError::ValueConversion`] instead.
//!
//! Number representation follows ECMAScript: every script `Number`
//! is an IEEE-754 `f64`. QuickJS internally distinguishes `Int`
//! and `Float` as a storage optimisation, but the host surface
//! collapses both to `ScriptValue::Number(f64)` — the test corpus
//! asserts against `Number(3.0)`, not against an int / float
//! discriminant, matching what authors will write in scenes.
//!
//! Object iteration order: rquickjs `Object::keys()` returns keys
//! in insertion order (QuickJS-preserving). Slice-1 surface is a
//! `Vec<(String, ScriptValue)>` rather than a `HashMap` to retain
//! that order across the host boundary; T2 corpus tests assert
//! against the script-author-visible key order.
//!
//! Slice-4+ may widen the value vocabulary as the
//! `EV-SCRIPT-CONTRACT` host-API surface lands (control bindings,
//! event payloads). The `#[non_exhaustive]` attribute keeps that
//! widening additive on the public surface.

/// Owned, host-side projection of a QuickJS execution result.
///
/// See module-level docs for the slice-1 vocabulary, the
/// `undefined` → `Null` lowering, the `Int` / `Float` collapse,
/// and the deliberate omissions (`Symbol`, `BigInt`, `Function`,
/// `Promise`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ScriptValue {
    /// Script `null` or `undefined`.
    Null,
    /// Script `boolean`.
    Bool(bool),
    /// Script `number`. ECMAScript collapses `Int` and `Float` into
    /// a single `Number` type; QuickJS's storage-level distinction
    /// is intentionally dropped at this boundary.
    Number(f64),
    /// Script `string`.
    String(String),
    /// Script array. Order matches the script-side index order.
    Array(Vec<ScriptValue>),
    /// Script plain object. Insertion order is preserved
    /// (QuickJS / rquickjs `Object::keys()` semantics).
    Object(Vec<(String, ScriptValue)>),
}
