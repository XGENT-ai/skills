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
    attach_brands(&mut diagram, input)?;
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

fn attach_brands(diagram: &mut CompiledDiagram, input: &DiagramInput) -> Result<(), Diagnostics> {
    let mut errors = vec![];
    for node in &mut diagram.nodes {
        let path = format!("{}/brand", node.authored.source_path);
        let Some(brand) = input.brands.get(&path) else {
            continue;
        };
        let Some(label) = node
            .text
            .iter_mut()
            .find(|text| text.source_path.ends_with("/label"))
        else {
            continue;
        };
        let required =
            crate::geometry::text_units_for_contract(&label.content, input.input_contract)
                * label.font_size
                * 0.6;
        if required > node.bounds.width - 48.0 || node.bounds.height < 28.0 {
            errors.push(Diagnostic::error(Some(input.diagram_type.as_str()),"brand/title-capacity",&path,"The brand title rail cannot fit the complete label at its current font size; widen the node without changing its pin"));
            continue;
        }
        label.position.x = node.bounds.x + node.bounds.width / 2.0 - 8.0;
        label.anchor = "middle".into();
        let rect = crate::scene::Rect {
            x: node.bounds.x + node.bounds.width - 24.0,
            y: label.position.y - label.font_size * 0.8 - 2.0,
            width: 16.0,
            height: 16.0,
        };
        if rect.y < node.bounds.y + 2.0
            || rect.y + rect.height > node.bounds.y + node.bounds.height - 2.0
        {
            errors.push(Diagnostic::error(
                Some(input.diagram_type.as_str()),
                "brand/title-capacity",
                &path,
                "The fixed title rail has insufficient height for a brand mark",
            ));
            continue;
        }
        if let Some(sigil) = node.sigil.as_mut() {
            let overlaps = |x: f64, y: f64| {
                let mark = crate::scene::Rect {
                    x,
                    y,
                    width: sigil.size,
                    height: sigil.size,
                };
                rects_overlap(&mark, &rect, 1.0)
                    || node.text.iter().any(|t| {
                        rects_overlap(
                            &mark,
                            &text_glyph_box(
                                &t.content,
                                t.position.x,
                                t.position.y,
                                t.font_size,
                                &t.anchor,
                                input.input_contract,
                            ),
                            1.0,
                        )
                    })
            };
            if overlaps(sigil.position.x, sigil.position.y) {
                let positions = [
                    (node.bounds.x + 4.0, node.bounds.y + 4.0),
                    (
                        node.bounds.x + 4.0,
                        node.bounds.y + node.bounds.height - sigil.size - 4.0,
                    ),
                    (
                        node.bounds.x + node.bounds.width - sigil.size - 4.0,
                        node.bounds.y + node.bounds.height - sigil.size - 4.0,
                    ),
                ];
                if let Some((x, y)) = positions.into_iter().find(|(x, y)| !overlaps(*x, *y)) {
                    sigil.position = crate::scene::Point { x, y };
                } else {
                    errors.push(Diagnostic::error(Some(input.diagram_type.as_str()),"brand/title-capacity",&path,"The fixed card cannot retain both brand and semantic mark without covering text"));
                    continue;
                }
            }
        }
        node.brand = Some(crate::scene::BrandPlacement {
            bounds: rect,
            resource: brand.clone(),
        });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
