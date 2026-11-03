//! impeccable-comp-verbs: the four comp-fidelity verb orchestrators
//! (`comp-spec`, `comp-diff`, `font-match`, `build-phase`), ported from the
//! skill's JS scripts of the same name.
//!
//! The browser stays out: font-match's headless rendering of font specimens
//! is injected by the CLI as a [`font_match::FontRenderer`].
//!
//! font-match reads the skill's shipped font-index catalog at run time; see
//! [`font_match`].

pub mod build_phase;
pub mod completion;
pub mod comp_diff;
pub mod comp_spec;
mod map_inspection;
pub mod font_match;
mod util;

use impeccable_common::Io;

/// `impeccable comp-spec ...`
pub fn run_comp_spec(argv: &[String], io: &mut Io) -> i32 {
    comp_spec::run(argv, io)
}

/// `impeccable comp-diff ...`
pub fn run_comp_diff(argv: &[String], io: &mut Io) -> i32 {
    comp_diff::run(argv, io)
}

/// `impeccable font-match ...`. `renderer` supplies the headless-browser
/// specimen rendering; pass [`font_match::NoRenderer`] where no browser is
/// available (the catalog/shortlist path owns the ranking then).
pub fn run_font_match(argv: &[String], io: &mut Io, renderer: &mut dyn font_match::FontRenderer) -> i32 {
    font_match::run(argv, io, renderer)
}

pub mod asset_capture;

pub mod entry_capture;
