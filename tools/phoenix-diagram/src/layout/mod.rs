pub mod architecture;
pub mod class;
pub mod dataflow;
pub mod erd;
pub mod lifecycle;
pub(crate) mod router;
pub mod sequence;
pub mod timeline;
pub mod tree;
pub mod waterfall;
pub mod workflow;

use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::{rects_overlap, text_glyph_box};
use crate::input::{CompileOptions, DiagramInput, DiagramType, InputContract};
use crate::scene::{CompiledDiagram, Node};

pub fn compile(
    input: &DiagramInput,
    options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    if options
        .input_contract
        .is_some_and(|contract| contract != input.input_contract)
        || options
            .quality
            .is_some_and(|quality| quality != input.quality)
        || options.viewer_revision != input.viewer_revision
    {
        return Err(vec![Diagnostic::error(
            Some(input.diagram_type.as_str()),
            "input/config-binding",
            "/meta",
            "Compile options differ from the validated input binding",
        )]);
    }
    let mut diagram = match input.diagram_type {
        DiagramType::Architecture => architecture::compile(input, options),
        DiagramType::Workflow => workflow::compile(input, options),
        DiagramType::Sequence => sequence::compile(input, options),
        DiagramType::Dataflow => dataflow::compile(input, options),
        DiagramType::Lifecycle => lifecycle::compile(input, options),
        DiagramType::Erd => erd::compile(input, options),
        DiagramType::Class => class::compile(input, options),
        DiagramType::Tree => tree::compile(input, options),
        DiagramType::Timeline => timeline::compile(input, options),
        DiagramType::Waterfall => waterfall::compile(input, options),
    }?;
    clear_sigil_labels(&mut diagram.nodes, input.input_contract);
    Ok(diagram)
}

/// The corner sigil is decorative. Where frozen geometry lets it touch the
/// primary label (legacy-2.17 sequence and narrow dataflow cards), it moves
/// into the card's top rail and shrinks to clear the glyphs; without room for
/// a 7px mark it is omitted. Card and text geometry are unchanged.
fn clear_sigil_labels(nodes: &mut [Node], contract: InputContract) {
    for node in nodes {
        let Some(sigil) = node.sigil.as_mut() else {
            continue;
        };
        let labels: Vec<_> = node
            .text
            .iter()
            .filter(|text| text.source_path.ends_with("/label"))
            .map(|text| {
                text_glyph_box(
                    &text.content,
                    text.position.x,
                    text.position.y,
                    text.font_size,
                    &text.anchor,
                    contract,
                )
            })
            .collect();
        let mark = |y: f64, size: f64| crate::scene::Rect {
            x: sigil.position.x,
            y,
            width: size,
            height: size,
        };
        if !labels
            .iter()
            .any(|label| rects_overlap(&mark(sigil.position.y, sigil.size), label, 1.0))
        {
            continue;
        }
        let top = node.bounds.y + 3.0;
        let room = labels
            .iter()
            .filter(|label| rects_overlap(&mark(top, sigil.size), label, 1.0))
            .map(|label| label.y - 1.0 - top)
            .fold(sigil.size, f64::min);
        if room >= 7.0 {
            sigil.position.y = top;
            sigil.size = room;
        } else {
            node.sigil = None;
        }
    }
}
