//! Context and utility verbs: context, doctor, pin, surface-brief,
//! critique-storage, embed-prompt, signals, detect-csp, concept-seed,
//! serve-question, compare, generate-image. Each verb is `run(args, io) -> exit code`.

pub mod artifact_schema;
pub mod context;
pub mod context_cli;
pub mod detect_csp;
pub mod hook_markers;
pub mod http;
pub mod jsp;
pub mod pin;
pub mod provider;
pub mod staleness;
pub mod staleness_notice;
pub mod surface_briefs;
pub mod target_args;
pub mod target_slug;
pub mod url;
pub mod util;

pub use context_cli::run as run_context;
pub use detect_csp::run as run_detect_csp;
pub use pin::run as run_pin;
pub mod critique_storage;
pub mod surface_brief_cli;
pub use critique_storage::run as run_critique_storage;
pub use surface_brief_cli::run as run_surface_brief;
pub mod embed_prompt;
pub use embed_prompt::{read_provenance, run as run_embed_prompt, write_provenance, Carried};
pub mod signals;
pub use signals::run as run_signals;
pub mod design_parser;
pub mod staleness_deep;
pub mod doctor;
pub use doctor::run as run_doctor;
pub mod catalog;
pub mod roll_selection;
pub mod seed_text;
pub mod concept_seed;
pub use concept_seed::run as run_concept_seed;
pub mod generate_image;
pub use generate_image::run as run_generate_image;
pub mod question_page;
pub mod serve_question;
pub use serve_question::run as run_serve_question;
pub mod compare;
pub use compare::run as run_compare;

pub mod component_review;

pub mod accepted_briefs;
pub mod context_json;
